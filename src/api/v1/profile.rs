use topcoat::{
    Result,
    context::Cx,
    router::{Body, BodyLimit, route},
};

use crate::api::body::MAX_JSON_BODY;
use crate::api::problem::ApiResponse;
use crate::api::registry::{serve, serve_with_body};
use crate::profile::api::{GetProfileAction, UpdateProfileAction};

#[route(GET "/api/v1/me")]
pub async fn get_profile_route(cx: &Cx) -> Result<ApiResponse> {
    serve::<GetProfileAction>(cx).await
}

#[route(PATCH "/api/v1/me", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn update_profile_route(cx: &Cx, body: Body) -> Result<ApiResponse> {
    serve_with_body::<UpdateProfileAction>(cx, body).await
}
