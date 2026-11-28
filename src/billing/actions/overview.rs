use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Serialize;
use time::OffsetDateTime;

use crate::action::{ActionResult, Caller, authorize};
use crate::billing::catalog::{FREE_PLAN_KEY, FREE_PLAN_NAME};
use crate::billing::queries::{
    query_get_current_subscription, query_get_customer_id, query_has_any_subscription,
};
use crate::billing::subscription::{Subscription, SubscriptionStatus};
use crate::env::Env;
use crate::permission::Permission;
use crate::time_zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
pub enum BillingStatus {
    #[serde(rename = "free")]
    Free,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "cancelling")]
    Cancelling,
    #[serde(rename = "past_due")]
    PastDue,
}

impl BillingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Active => SubscriptionStatus::Active.as_str(),
            Self::Cancelling => SubscriptionStatus::Cancelling.as_str(),
            Self::PastDue => SubscriptionStatus::PastDue.as_str(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Active => SubscriptionStatus::Active.label(),
            Self::Cancelling => SubscriptionStatus::Cancelling.label(),
            Self::PastDue => SubscriptionStatus::PastDue.label(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(description = "The account's plan and where its subscription stands.")]
pub struct Billing {
    #[schemars(description = "The plan's key: `free`, or a paid plan's key such as `pro`.")]
    pub plan: String,
    #[schemars(description = "The plan's display name, e.g. Pro.")]
    pub plan_name: String,
    #[schemars(description = "free, active, cancelling (paid until ends_on) or past_due.")]
    pub status: BillingStatus,
    #[schemars(description = "Next renewal (YYYY-MM-DD in the account's time zone), or null.")]
    pub renews_on: Option<String>,
    #[schemars(description = "When a cancelled plan ends (YYYY-MM-DD), or null.")]
    pub ends_on: Option<String>,
    #[serde(skip)]
    pub renews_at: Option<OffsetDateTime>,
    #[serde(skip)]
    pub ends_at: Option<OffsetDateTime>,
    #[serde(skip)]
    pub configured: bool,
    #[serde(skip)]
    pub ever_paid: bool,
}

pub async fn get_billing(env: &Env<'_>, caller: &Caller) -> ActionResult<Billing, Infallible> {
    authorize!(caller.require(Permission::ViewBilling));
    let user_id = caller.owner_id();
    let current = query_get_current_subscription(user_id, env.db()).await?;
    let ever_paid = query_has_any_subscription(user_id, env.db()).await?;
    let mut billing = billing_of(current.as_ref(), &caller.user.zone());
    billing.configured = env.stripe().is_some();
    billing.ever_paid = ever_paid;

    Ok(Ok(billing))
}

pub(in crate::billing) fn billing_of(current: Option<&Subscription>, zone: &Zone) -> Billing {
    let Some(subscription) = current else {
        return Billing {
            plan: FREE_PLAN_KEY.to_string(),
            plan_name: FREE_PLAN_NAME.to_string(),
            status: BillingStatus::Free,
            renews_on: None,
            ends_on: None,
            renews_at: None,
            ends_at: None,
            configured: false,
            ever_paid: false,
        };
    };
    let status = match subscription.status {
        SubscriptionStatus::Cancelling => BillingStatus::Cancelling,
        SubscriptionStatus::PastDue => BillingStatus::PastDue,
        SubscriptionStatus::Active | SubscriptionStatus::Ended => BillingStatus::Active,
    };
    let ends_at = subscription.ends_at;
    let renews_at = ends_at.is_none().then_some(subscription.current_period_end);
    Billing {
        plan: subscription.plan_key.clone(),
        plan_name: subscription.plan_name.clone(),
        status,
        renews_on: renews_at.map(|at| zone.iso_date(at)),
        ends_on: ends_at.map(|at| zone.iso_date(at)),
        renews_at,
        ends_at,
        configured: false,
        ever_paid: true,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminBilling {
    pub plan_name: String,
    pub status: BillingStatus,
    pub stripe_url: Option<String>,
}

pub async fn admin_billing(
    env: &Env<'_>,
    caller: &Caller,
    user_id: i32,
) -> ActionResult<AdminBilling, Infallible> {
    authorize!(caller.require(Permission::AdministerUsers));
    let current = query_get_current_subscription(user_id, env.db()).await?;
    let customer = query_get_customer_id(user_id, env.db()).await?;
    let billing = billing_of(current.as_ref(), &Zone::utc());
    let test_mode = env.stripe().is_some_and(|stripe| stripe.is_test_mode());
    let stripe_url = customer.map(|id| dashboard_url(&id, test_mode));

    Ok(Ok(AdminBilling {
        plan_name: billing.plan_name,
        status: billing.status,
        stripe_url,
    }))
}

fn dashboard_url(customer_id: &str, test_mode: bool) -> String {
    let mode = if test_mode { "test/" } else { "" };
    format!("https://dashboard.stripe.com/{mode}customers/{customer_id}")
}
