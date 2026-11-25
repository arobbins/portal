use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::action::{ActionError, ActionProblem, Denied};
use crate::api::scope::ApiScope;
use crate::api_tokens::testing::issue_token;
use crate::billing::actions::{CheckoutProblem, StartCheckout, start_checkout};
use crate::billing::testing::{FakeStripe, unreachable_stripe};

fn pro() -> StartCheckout {
    StartCheckout {
        price: "price_pro".to_string(),
    }
}

async fn token_caller(user: &Arc<User>, scope: ApiScope, db: &PgPool) -> Caller {
    let token = issue_token(user.id, &[scope.as_str()], time::Duration::days(1), db).await;
    Caller::api_token(user.clone(), token.id, vec![scope])
}

#[sqlx::test]
async fn checkout_uses_automatic_payment_methods_and_reuses_the_customer(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = active_user("chooser@example.test", &db).await;
    let caller = Caller::browser(user.clone());

    let first = start_checkout(&app.env(None), &caller, &pro())
        .await
        .unwrap()
        .unwrap();
    let second = start_checkout(&app.env(None), &caller, &pro())
        .await
        .unwrap()
        .unwrap();

    assert_ne!(first.checkout_url, second.checkout_url);
    assert_eq!(stripe.seen_at("POST", "/v1/customers").len(), 1);
    let sessions = stripe.seen_at("POST", "/v1/checkout/sessions");
    assert_eq!(sessions.len(), 2);
    let session = &sessions[0];
    assert_eq!(session.field("mode"), Some("subscription"));
    assert_eq!(session.field("customer"), Some(CUSTOMER));
    assert_eq!(session.field("line_items[0][price]"), Some("price_pro"));
    assert!(
        session
            .form
            .iter()
            .all(|(key, _)| !key.starts_with("payment_method_types"))
    );
    assert!(activity_of(&app, &user).await.is_empty());
}

#[sqlx::test]
async fn a_repeated_idempotency_key_returns_the_same_link(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = active_user("retry@example.test", &db).await;
    let caller = token_caller(&user, ApiScope::BillingWrite, &db).await;
    let env = app.env(None).with_idempotency_key(Some("order-42"));

    let first = start_checkout(&env, &caller, &pro())
        .await
        .unwrap()
        .unwrap();
    let again = start_checkout(&env, &caller, &pro())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(first, again);
    let sessions = stripe.seen_at("POST", "/v1/checkout/sessions");
    assert_eq!(sessions.len(), 1);
    assert!(
        sessions[0]
            .idempotency_key
            .as_deref()
            .unwrap()
            .ends_with("order-42")
    );
    assert_eq!(
        activity_of(&app, &user).await,
        ["Checkout for Pro started by token Test token"]
    );
}

#[sqlx::test]
async fn an_idempotency_key_is_checked_and_belongs_to_one_price(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = active_user("keys@example.test", &db).await;
    let caller = Caller::browser(user);
    let env = app.env(None).with_idempotency_key(Some("order-42"));
    start_checkout(&env, &caller, &pro())
        .await
        .unwrap()
        .unwrap();

    let business = StartCheckout {
        price: "price_business".to_string(),
    };
    assert_eq!(
        start_checkout(&env, &caller, &business).await.unwrap(),
        Err(ActionError::Problem(CheckoutProblem::IdempotencyKeyReused))
    );
    let bad = app.env(None).with_idempotency_key(Some("not ok"));
    assert_eq!(
        start_checkout(&bad, &caller, &pro()).await.unwrap(),
        Err(ActionError::Problem(CheckoutProblem::InvalidIdempotencyKey))
    );
}

#[sqlx::test]
async fn only_billing_write_tokens_may_start_a_checkout(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = active_user("scoped@example.test", &db).await;
    let env = app.env(None);

    for scope in [ApiScope::BillingRead, ApiScope::ProfileWrite] {
        let caller = token_caller(&user, scope, &db).await;
        assert_eq!(
            start_checkout(&env, &caller, &pro()).await.unwrap(),
            Err(ActionError::Denied(Denied::MissingScope(
                ApiScope::BillingWrite
            )))
        );
    }
    let reader = token_caller(&user, ApiScope::BillingRead, &db).await;
    assert!(
        super::super::get_billing(&env, &reader)
            .await
            .unwrap()
            .is_ok()
    );
    assert!(stripe.seen_at("POST", "/v1/checkout/sessions").is_empty());
}

#[sqlx::test]
async fn a_subscribed_account_is_refused_first_and_unknown_prices_are_refused(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("subscribed@example.test", CUSTOMER, &db).await;
    let caller = Caller::browser(user.clone());
    let env = app.env(None);
    let unknown = StartCheckout {
        price: "price_addon".to_string(),
    };

    assert_eq!(
        start_checkout(&env, &caller, &unknown).await.unwrap(),
        Err(ActionError::Problem(CheckoutProblem::UnknownPrice))
    );
    deliver(
        &app,
        &subscription(
            "customer.subscription.created",
            "evt_1",
            1_800_000_000,
            "active",
            false,
        ),
    )
    .await;

    assert_eq!(
        start_checkout(&env, &caller, &unknown).await.unwrap(),
        Err(ActionError::Problem(CheckoutProblem::AlreadySubscribed))
    );
    assert_eq!(CheckoutProblem::AlreadySubscribed.status().as_u16(), 409);
    assert_eq!(CheckoutProblem::UnknownPrice.code(), "unknown_price");
}

#[sqlx::test]
async fn an_unreachable_stripe_changes_nothing(db: PgPool) {
    let app = unreachable_stripe(&db);
    let user = active_user("offline@example.test", &db).await;

    assert_eq!(
        start_checkout(&app.env(None), &Caller::browser(user.clone()), &pro())
            .await
            .unwrap(),
        Err(ActionError::Problem(CheckoutProblem::Unavailable))
    );
    assert_eq!(CheckoutProblem::Unavailable.status().as_u16(), 503);
    assert!(activity_of(&app, &user).await.is_empty());
}

#[test]
fn the_checkout_input_is_one_required_price() {
    let schema = schemars::schema_for!(StartCheckout);
    let value = schema.as_value();

    assert_eq!(value["required"], serde_json::json!(["price"]));
    assert_eq!(value["additionalProperties"], false);
}
