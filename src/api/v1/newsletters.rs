use topcoat::{
    Result,
    context::Cx,
    router::{Body, BodyLimit, route},
};

use crate::api::body::MAX_JSON_BODY;
use crate::api::problem::ApiResponse;
use crate::api::serve::{serve, serve_with_body};
use crate::preferences::api::{GetNewslettersAction, UpdateNewslettersAction};

#[route(GET "/api/v1/newsletters")]
pub async fn get_newsletters_route(cx: &Cx) -> Result<ApiResponse> {
    serve::<GetNewslettersAction>(cx).await
}

#[route(PATCH "/api/v1/newsletters", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn update_newsletters_route(cx: &Cx, body: Body) -> Result<ApiResponse> {
    serve_with_body::<UpdateNewslettersAction>(cx, body).await
}
