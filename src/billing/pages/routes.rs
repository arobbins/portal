use axum::http::{HeaderMap, HeaderValue, header::CONTENT_TYPE};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, BodyLimit, StatusCode,
        content::Form,
        error::{SeeOther, see_other},
        request::{self, Bytes, FromRequest},
        route,
    },
};

use super::checkout_pending::mark_checkout_pending;
use crate::account::tabs::AccountPage;
use crate::action::{ActionProblem, ProblemSpec, require_browser_caller};
use crate::billing::actions::{
    CheckoutProblem, OpenPortal, PortalProblem, StartCheckout, open_billing_portal,
    receive_stripe_event, start_checkout,
};
use crate::flash::{Flash, set_flash};
use crate::routes::PRICING_PATH;
use crate::web::{env, session};

const MAX_WEBHOOK_BODY: usize = 512 * 1024;
const BILLING_PROBLEM: &str = "billing_problem";

fn known_problems() -> impl Iterator<Item = ProblemSpec> {
    CheckoutProblem::ALL
        .into_iter()
        .map(ActionProblem::spec)
        .chain(PortalProblem::ALL.into_iter().map(ActionProblem::spec))
}

async fn back_with_problem(cx: &Cx, path: String, problem: ProblemSpec) -> Result<SeeOther> {
    session(cx)?.insert(BILLING_PROBLEM, problem.code).await?;
    Ok(see_other(path))
}

pub async fn take_billing_problem(cx: &Cx) -> Result<Option<ProblemSpec>> {
    let code = session(cx)?.remove::<String>(BILLING_PROBLEM).await?;
    Ok(code.and_then(|code| known_problems().find(|spec| spec.code == code)))
}

#[route(POST "/account/checkout/start")]
pub async fn start_checkout_submit(cx: &Cx, Form(input): Form<StartCheckout>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    match start_checkout(&env(cx), &caller, &input).await? {
        Ok(link) => Ok(see_other(link.checkout_url)),
        Err(error) => {
            let problem = error.for_browser()?.spec();
            back_with_problem(cx, PRICING_PATH.to_string(), problem).await
        }
    }
}

#[route(POST "/account/billing-portal/open")]
pub async fn open_portal_submit(cx: &Cx, Form(input): Form<OpenPortal>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    match open_billing_portal(&env(cx), &caller, &input).await? {
        Ok(link) => Ok(see_other(link.url)),
        Err(error) => {
            let problem = error.for_browser()?.spec();
            back_with_problem(cx, input.from.path(), problem).await
        }
    }
}

#[route(GET "/account/checkout/paid")]
pub async fn checkout_paid(cx: &Cx) -> Result<SeeOther> {
    require_browser_caller(cx)?;
    mark_checkout_pending(&session(cx)?, env(cx).now()).await?;
    Ok(see_other(AccountPage::Plan.path()))
}

#[route(GET "/account/checkout/cancelled")]
pub async fn checkout_cancelled(cx: &Cx) -> Result<SeeOther> {
    require_browser_caller(cx)?;
    set_flash(&session(cx)?, Flash::CheckoutCancelled).await?;
    Ok(see_other(AccountPage::Plan.path()))
}

#[route(POST "/webhooks/stripe", with(BodyLimit::max(MAX_WEBHOOK_BODY)))]
pub async fn stripe_webhook(cx: &Cx, body: Body) -> Result<(StatusCode, HeaderMap, String)> {
    let signature = request::headers(cx)
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let Ok(payload) = Bytes::from_request(cx, body).await else {
        return Ok(reply(StatusCode::PAYLOAD_TOO_LARGE));
    };
    let outcome = receive_stripe_event(&env(cx), signature.as_deref(), &payload).await?;
    Ok(reply(outcome.status()))
}

fn reply(status: StatusCode) -> (StatusCode, HeaderMap, String) {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let body = serde_json::json!({ "received": status.is_success() }).to_string();
    (status, headers, body)
}
