use std::convert::Infallible;

use topcoat::router::StatusCode;

use crate::action::{ActionError, ActionResult, Caller, Outcome};
use crate::api::catalog::{ErrorDoc, HttpMethod};
use crate::api::registry::{ApiAction, NoInput, PublicApiAction};
use crate::billing::actions::{
    Billing, BillingStatus, CheckoutLink, CheckoutProblem, PlanList, PlanPrice, PlansProblem,
    StartCheckout, get_billing, list_plans, start_checkout,
};
use crate::env::Env;
use crate::permission::Permission;

const GROUP: &str = "Billing";

pub struct ListPlansAction;

impl ApiAction for ListPlansAction {
    const ID: &'static str = "list-plans";
    const GROUP: &'static str = GROUP;
    const METHOD: HttpMethod = HttpMethod::Get;
    const PATH: &'static str = "/api/v1/billing/plans";
    const SUMMARY: &'static str = "List the plans";
    const DESCRIPTION: &'static str = "Lists every paid plan's prices, cheapest first. Every account starts on Free, which isn't listed. Pass a price id to start-checkout to put the account on that plan.";
    const PERMISSION: Permission = Permission::ViewPlans;

    type Input = NoInput;
    type Output = PlanList;
    type Problem = PlansProblem;

    async fn run(
        env: &Env<'_>,
        _caller: &Caller,
        _input: &NoInput,
    ) -> ActionResult<PlanList, PlansProblem> {
        Ok(list_plans(env).await?.map_err(ActionError::Problem))
    }

    fn example_input() -> Option<NoInput> {
        None
    }

    fn example_output() -> PlanList {
        PlanList {
            data: vec![PlanPrice {
                plan: "pro".to_string(),
                plan_name: "Pro".to_string(),
                price: "price_1QPro".to_string(),
                amount: 1200,
                currency: "usd".to_string(),
                interval: "month".to_string(),
                label: String::new(),
                decimal_amount: String::new(),
                period: None,
            }],
            next_cursor: None,
        }
    }

    fn problems() -> Vec<ErrorDoc> {
        PlansProblem::ALL.into_iter().map(ErrorDoc::of).collect()
    }
}

impl PublicApiAction for ListPlansAction {
    async fn run_public(env: &Env<'_>, _input: &NoInput) -> Outcome<PlanList, PlansProblem> {
        list_plans(env).await
    }
}

pub struct GetBillingAction;

impl ApiAction for GetBillingAction {
    const ID: &'static str = "get-billing";
    const GROUP: &'static str = GROUP;
    const METHOD: HttpMethod = HttpMethod::Get;
    const PATH: &'static str = "/api/v1/billing";
    const SUMMARY: &'static str = "Get your plan";
    const DESCRIPTION: &'static str = "Returns the account's plan and its status: free, active, cancelling (paid until ends_on, then Free) or past_due (the last renewal failed; the plan stays until Stripe ends it). Dates are in the account's time zone.";
    const PERMISSION: Permission = Permission::ViewBilling;

    type Input = NoInput;
    type Output = Billing;
    type Problem = Infallible;

    async fn run(
        env: &Env<'_>,
        caller: &Caller,
        _input: &NoInput,
    ) -> ActionResult<Billing, Infallible> {
        get_billing(env, caller).await
    }

    fn example_input() -> Option<NoInput> {
        None
    }

    fn example_output() -> Billing {
        Billing {
            plan: "pro".to_string(),
            plan_name: "Pro".to_string(),
            status: BillingStatus::Active,
            renews_on: Some("2026-12-25".to_string()),
            ends_on: None,
            renews_at: None,
            ends_at: None,
            configured: true,
            ever_paid: true,
        }
    }

    fn problems() -> Vec<ErrorDoc> {
        Vec::new()
    }
}

pub struct StartCheckoutAction;

impl ApiAction for StartCheckoutAction {
    const ID: &'static str = "start-checkout";
    const GROUP: &'static str = GROUP;
    const METHOD: HttpMethod = HttpMethod::Post;
    const PATH: &'static str = "/api/v1/billing/checkout";
    const SUMMARY: &'static str = "Start a checkout";
    const DESCRIPTION: &'static str = "Creates a Stripe Checkout link for the token's account and one offered price. Give the link to the person (or their wallet) to pay; nothing is bought until they do, and the plan changes once Stripe confirms the payment. Refused while the account already has a subscription. Send an Idempotency-Key header to get the same link back when you retry. Records \"Checkout started\" in the account's activity.";
    const PERMISSION: Permission = Permission::StartCheckout;
    const SUCCESS: StatusCode = StatusCode::CREATED;

    type Input = StartCheckout;
    type Output = CheckoutLink;
    type Problem = CheckoutProblem;

    async fn run(
        env: &Env<'_>,
        caller: &Caller,
        input: &StartCheckout,
    ) -> ActionResult<CheckoutLink, CheckoutProblem> {
        start_checkout(env, caller, input).await
    }

    fn example_input() -> Option<StartCheckout> {
        Some(StartCheckout {
            price: "price_1QPro".to_string(),
        })
    }

    fn example_output() -> CheckoutLink {
        CheckoutLink {
            checkout_url: "https://checkout.stripe.com/c/pay/cs_test_a1b2c3".to_string(),
        }
    }

    fn problems() -> Vec<ErrorDoc> {
        CheckoutProblem::ALL.into_iter().map(ErrorDoc::of).collect()
    }
}
