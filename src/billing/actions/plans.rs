use schemars::JsonSchema;
use serde::Serialize;

use super::{Unavailable, too_many_attempts};
use crate::action::{ActionProblem, ListedProblem, Outcome, ProblemSpec, fail};
use crate::billing::catalog::{PaidPrice, load_catalog};
use crate::billing::period::{BillingPeriod, Interval};
use crate::env::Env;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(description = "One price of a paid plan. Free isn't listed: it needs no Checkout.")]
pub struct PlanPrice {
    #[schemars(description = "The plan's stable key, e.g. pro.")]
    pub plan: String,
    #[schemars(description = "The plan's display name, e.g. Pro.")]
    pub plan_name: String,
    #[schemars(description = "The Stripe price id to pass to start_checkout (price_…).")]
    pub price: String,
    #[schemars(description = "The price in the currency's smallest unit (cents).")]
    pub amount: i64,
    #[schemars(description = "Lowercase ISO currency code, e.g. usd.")]
    pub currency: String,
    #[schemars(description = "How often it's charged: day, week, month or year.")]
    pub interval: Interval,
    #[serde(skip)]
    pub label: String,
    #[serde(skip)]
    pub decimal_amount: String,
    #[serde(skip)]
    pub period: Option<BillingPeriod>,
}

impl PlanPrice {
    fn of(price: &PaidPrice) -> Self {
        Self {
            plan: price.plan_key.clone(),
            plan_name: price.plan_name.clone(),
            price: price.price_id.clone(),
            amount: price.amount,
            currency: price.currency.clone(),
            interval: price.interval,
            label: price.label(),
            decimal_amount: price.decimal_amount(),
            period: BillingPeriod::of_interval(price.interval, price.interval_count),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(description = "The paid plans' prices, cheapest first.")]
pub struct PlanList {
    #[schemars(description = "Every price on offer.")]
    pub data: Vec<PlanPrice>,
    #[schemars(description = "Always null: every price fits on one page.")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlansProblem {
    Unavailable,
    TooManyRequests,
}

impl PlansProblem {
    pub const ALL: [Self; 2] = [Self::Unavailable, Self::TooManyRequests];
}

impl ListedProblem for PlansProblem {
    fn all() -> &'static [Self] {
        &Self::ALL
    }
}

impl ActionProblem for PlansProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Unavailable => Unavailable.spec(),
            Self::TooManyRequests => too_many_attempts(),
        }
    }
}

pub async fn list_plans(env: &Env<'_>) -> Outcome<PlanList, PlansProblem> {
    if !env.limits().allow_plan_list(env.ip).await? {
        return fail(PlansProblem::TooManyRequests);
    }
    let Some(stripe) = env.stripe() else {
        return Ok(Ok(PlanList {
            data: Vec::new(),
            next_cursor: None,
        }));
    };
    let catalog = match load_catalog(stripe).await {
        Ok(catalog) => catalog,
        Err(error) => {
            tracing::warn!("plans unavailable: {error}");
            return fail(PlansProblem::Unavailable);
        }
    };

    Ok(Ok(PlanList {
        data: catalog.prices.iter().map(PlanPrice::of).collect(),
        next_cursor: None,
    }))
}
