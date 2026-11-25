use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use topcoat::router::StatusCode;

use super::customer::ensure_customer;
use super::{Unavailable, too_many_attempts};
use crate::action::{ActionProblem, ActionResult, Caller, Outcome, ProblemSpec, authorize, fail};
use crate::api_tokens::lifecycle::token_name;
use crate::billing::catalog::{PaidPrice, is_price_id, load_catalog};
use crate::billing::queries::{
    CheckoutRequest, query_get_checkout_request, query_get_current_subscription,
    query_insert_checkout_request,
};
use crate::env::Env;
use crate::permission::Permission;
use crate::routes::{CHECKOUT_CANCELLED_PATH, CHECKOUT_PAID_PATH};
use crate::stripe::{Request, Stripe, StripeError};
use crate::user::events::{AccountEventKind, BillingEventKind, EventSource, record_event};

pub const PRICE_FIELD: &str = "price";
pub const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 255;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Start a Stripe Checkout for one of the offered prices.")]
pub struct StartCheckout {
    #[schemars(
        description = "The Stripe price id to subscribe to (price_…), from the plans list.",
        length(max = 255)
    )]
    pub price: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(description = "A Stripe Checkout link for the token's account.")]
pub struct CheckoutLink {
    #[schemars(
        description = "Open this Stripe-hosted page to pay; the plan changes once Stripe confirms the payment."
    )]
    pub checkout_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckoutProblem {
    AlreadySubscribed,
    UnknownPrice,
    InvalidIdempotencyKey,
    IdempotencyKeyReused,
    TooManyAttempts,
    Unavailable,
}

impl CheckoutProblem {
    pub const ALL: [Self; 6] = [
        Self::AlreadySubscribed,
        Self::UnknownPrice,
        Self::InvalidIdempotencyKey,
        Self::IdempotencyKeyReused,
        Self::TooManyAttempts,
        Self::Unavailable,
    ];
}

impl ActionProblem for CheckoutProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::AlreadySubscribed => ProblemSpec::new(
                "already_subscribed",
                "This account already has a subscription. Change or cancel it with Manage billing in the app.",
            )
            .with_status(StatusCode::CONFLICT),
            Self::UnknownPrice => ProblemSpec::new(
                "unknown_price",
                "That price isn't offered. List the plans and pass one of their price ids.",
            )
            .on_field(PRICE_FIELD),
            Self::InvalidIdempotencyKey => ProblemSpec::new(
                "invalid_idempotency_key",
                "The Idempotency-Key header must be 1 to 255 visible ASCII characters.",
            )
            .with_status(StatusCode::BAD_REQUEST),
            Self::IdempotencyKeyReused => ProblemSpec::new(
                "idempotency_key_reused",
                "This Idempotency-Key was already used for a different price. Use a new key.",
            ),
            Self::TooManyAttempts => too_many_attempts(),
            Self::Unavailable => Unavailable.spec(),
        }
    }
}

pub async fn start_checkout(
    env: &Env<'_>,
    caller: &Caller,
    input: &StartCheckout,
) -> ActionResult<CheckoutLink, CheckoutProblem> {
    authorize!(caller.require(Permission::StartCheckout));
    let Some(stripe) = env.stripe() else {
        return fail(CheckoutProblem::Unavailable);
    };
    if query_get_current_subscription(caller.owner_id(), env.db())
        .await?
        .is_some()
    {
        return fail(CheckoutProblem::AlreadySubscribed);
    }
    let key = match check_key(env, caller, input).await? {
        Ok(Key::Replay(url)) => return Ok(Ok(CheckoutLink { checkout_url: url })),
        Ok(Key::Fresh(key)) => key,
        Err(problem) => return fail(problem),
    };
    let price = match check_price(env, stripe, caller, input).await? {
        Ok(price) => price,
        Err(problem) => return fail(problem),
    };
    let url = match create_session(env, stripe, caller, &price, key.as_deref()).await? {
        Ok(url) => url,
        Err(error) => {
            tracing::warn!("checkout unavailable: {error}");
            return fail(CheckoutProblem::Unavailable);
        }
    };
    store_checkout(env, caller, &price, key.as_deref(), &url).await?;

    Ok(Ok(CheckoutLink { checkout_url: url }))
}

enum Key {
    Replay(String),
    Fresh(Option<String>),
}

async fn check_key(
    env: &Env<'_>,
    caller: &Caller,
    input: &StartCheckout,
) -> Outcome<Key, CheckoutProblem> {
    let Some(key) = env.idempotency_key else {
        return Ok(Ok(Key::Fresh(None)));
    };
    let valid = !key.is_empty()
        && key.len() <= MAX_IDEMPOTENCY_KEY_LENGTH
        && key.bytes().all(|byte| byte.is_ascii_graphic());
    if !valid {
        return fail(CheckoutProblem::InvalidIdempotencyKey);
    }
    match query_get_checkout_request(caller.owner_id(), key, env.db()).await? {
        Some(stored) if stored.price_id == input.price => Ok(Ok(Key::Replay(stored.checkout_url))),
        Some(_) => fail(CheckoutProblem::IdempotencyKeyReused),
        None => Ok(Ok(Key::Fresh(Some(key.to_string())))),
    }
}

async fn check_price(
    env: &Env<'_>,
    stripe: &Stripe,
    caller: &Caller,
    input: &StartCheckout,
) -> Outcome<PaidPrice, CheckoutProblem> {
    if !env
        .limits()
        .allow_billing_session(caller.owner_id())
        .await?
    {
        return fail(CheckoutProblem::TooManyAttempts);
    }
    if !is_price_id(&input.price) {
        return fail(CheckoutProblem::UnknownPrice);
    }
    match load_catalog(stripe).await {
        Ok(catalog) => match catalog.price(&input.price) {
            Some(price) => Ok(Ok(price.clone())),
            None => fail(CheckoutProblem::UnknownPrice),
        },
        Err(error) => {
            tracing::warn!("checkout unavailable: {error}");
            fail(CheckoutProblem::Unavailable)
        }
    }
}

async fn create_session(
    env: &Env<'_>,
    stripe: &Stripe,
    caller: &Caller,
    price: &PaidPrice,
    key: Option<&str>,
) -> topcoat::Result<Result<String, StripeError>> {
    let customer = match ensure_customer(env, stripe, &caller.user).await? {
        Ok(customer) => customer,
        Err(error) => return Ok(Err(error)),
    };
    let success = env.app.url(CHECKOUT_PAID_PATH);
    let cancel = env.app.url(CHECKOUT_CANCELLED_PATH);
    let user = caller.user.public_id.as_str();
    let form = [
        ("mode", "subscription"),
        ("customer", customer.as_str()),
        ("line_items[0][price]", price.price_id.as_str()),
        ("line_items[0][quantity]", "1"),
        ("success_url", success.as_str()),
        ("cancel_url", cancel.as_str()),
        ("client_reference_id", user),
        ("metadata[user]", user),
        ("subscription_data[metadata][user]", user),
    ];
    let scoped_key = key.map(|key| format!("checkout-{user}-{key}"));
    let request = Request::post("/v1/checkout/sessions", &form).idempotent(scoped_key.as_deref());
    let session = match stripe.send(request).await {
        Ok(session) => session,
        Err(error) => return Ok(Err(error)),
    };
    Ok(session["url"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| StripeError::Unreachable("Stripe returned no Checkout URL".to_string())))
}

async fn store_checkout(
    env: &Env<'_>,
    caller: &Caller,
    price: &PaidPrice,
    key: Option<&str>,
    url: &str,
) -> topcoat::Result<()> {
    let token = match caller.api_token_id() {
        Some(id) => token_name(id, caller.owner_id(), env.db()).await?,
        None => None,
    };
    let mut detail =
        json!({ "plan": price.plan_key, "plan_name": price.plan_name, "price": price.price_id });
    if let Some(token) = token {
        detail["token"] = json!(token);
    }
    let mut tx = env.db().begin().await?;
    if let Some(key) = key {
        let request = CheckoutRequest {
            price_id: price.price_id.clone(),
            checkout_url: url.to_string(),
        };
        query_insert_checkout_request(caller.owner_id(), key, &request, &mut *tx).await?;
    }
    let source = EventSource::of(caller);
    record_event(
        env,
        &source,
        AccountEventKind::Billing(BillingEventKind::CheckoutStarted),
        detail,
        &mut *tx,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
