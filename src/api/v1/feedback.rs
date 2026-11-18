use topcoat::{
    Result,
    context::Cx,
    router::{Body, BodyLimit, route},
};

use crate::api::body::MAX_JSON_BODY;
use crate::api::problem::ApiResponse;
use crate::api::registry::serve_with_body;
use crate::feedback::api::SendFeedbackAction;

#[route(POST "/api/v1/feedback", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn send_feedback_route(cx: &Cx, body: Body) -> Result<ApiResponse> {
    serve_with_body::<SendFeedbackAction>(cx, body).await
}
