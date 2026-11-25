use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use topcoat::router::StatusCode;

use super::{Unavailable, too_many_attempts};
use crate::account::tabs::AccountPage;
use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::billing::queries::query_get_customer_id;
use crate::env::Env;
use crate::permission::Permission;
use crate::routes::PRICING_PATH;
use crate::stripe::{Request, Stripe, StripeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum BillingPage {
    #[serde(rename = "pricing")]
    Pricing,
    #[serde(rename = "plan")]
    Plan,
    #[serde(rename = "invoices")]
    Invoices,
}

impl BillingPage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pricing => "pricing",
            Self::Plan => "plan",
            Self::Invoices => "invoices",
        }
    }

    pub fn path(self) -> String {
        match self {
            Self::Pricing => PRICING_PATH.to_string(),
            Self::Plan => AccountPage::Plan.path(),
            Self::Invoices => AccountPage::Invoices.path(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Open Stripe's billing portal for the account's own Stripe customer.")]
pub struct OpenPortal {
    #[schemars(description = "The page to come back to when leaving the portal.")]
    pub from: BillingPage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortalLink {
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalProblem {
    NothingToManage,
    TooManyAttempts,
    Unavailable,
}

impl PortalProblem {
    pub const ALL: [Self; 3] = [
        Self::NothingToManage,
        Self::TooManyAttempts,
        Self::Unavailable,
    ];
}

impl ActionProblem for PortalProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::NothingToManage => ProblemSpec::new(
                "no_billing_account",
                "There's nothing to manage yet. Choose a plan first.",
            )
            .with_status(StatusCode::CONFLICT),
            Self::TooManyAttempts => too_many_attempts(),
            Self::Unavailable => Unavailable.spec(),
        }
    }
}

pub async fn open_billing_portal(
    env: &Env<'_>,
    caller: &Caller,
    input: &OpenPortal,
) -> ActionResult<PortalLink, PortalProblem> {
    authorize!(caller.require(Permission::ManageBilling));
    let Some(stripe) = env.stripe() else {
        return fail(PortalProblem::Unavailable);
    };
    let Some(customer) = query_get_customer_id(caller.owner_id(), env.db()).await? else {
        return fail(PortalProblem::NothingToManage);
    };
    if !env
        .limits()
        .allow_billing_session(caller.owner_id())
        .await?
    {
        return fail(PortalProblem::TooManyAttempts);
    }
    let return_url = env.app.url(&input.from.path());
    match portal_session(stripe, &customer, &return_url).await {
        Ok(url) => Ok(Ok(PortalLink { url })),
        Err(error) => {
            tracing::warn!("billing portal unavailable: {error}");
            fail(PortalProblem::Unavailable)
        }
    }
}

async fn portal_session(
    stripe: &Stripe,
    customer: &str,
    return_url: &str,
) -> Result<String, StripeError> {
    let form = [("customer", customer), ("return_url", return_url)];
    let session = stripe
        .send(Request::post("/v1/billing_portal/sessions", &form))
        .await?;
    session["url"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| StripeError::Unreachable("Stripe returned no portal URL".to_string()))
}
