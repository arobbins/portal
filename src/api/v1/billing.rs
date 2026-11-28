use topcoat::{
    Result,
    context::Cx,
    router::{Body, BodyLimit, route},
};

use crate::api::body::MAX_JSON_BODY;
use crate::api::problem::ApiResponse;
use crate::api::serve::{serve, serve_public, serve_with_body};
use crate::billing::api::{GetBillingAction, ListPlansAction, StartCheckoutAction};

#[route(GET "/api/v1/billing/plans")]
pub async fn list_plans_route(cx: &Cx) -> Result<ApiResponse> {
    serve_public::<ListPlansAction>(cx).await
}

#[route(GET "/api/v1/billing")]
pub async fn get_billing_route(cx: &Cx) -> Result<ApiResponse> {
    serve::<GetBillingAction>(cx).await
}

#[route(POST "/api/v1/billing/checkout", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn start_checkout_route(cx: &Cx, body: Body) -> Result<ApiResponse> {
    serve_with_body::<StartCheckoutAction>(cx, body).await
}
