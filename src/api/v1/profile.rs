use topcoat::{
    Result,
    context::Cx,
    router::{Body, BodyLimit, route},
};

use crate::api::body::MAX_JSON_BODY;
use crate::api::handler::{api_call, api_call_with_body};
use crate::api::problem::{ApiResponse, respond};
use crate::profile::actions::{UpdateProfile, get_profile, update_profile};

#[route(GET "/api/v1/me")]
pub async fn get_profile_route(cx: &Cx) -> Result<ApiResponse> {
    api_call(cx, async |caller| Ok(respond(get_profile(&caller)))).await
}

#[route(PATCH "/api/v1/me", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn update_profile_route(cx: &Cx, body: Body) -> Result<ApiResponse> {
    api_call_with_body(cx, body, async |caller, input: UpdateProfile| {
        Ok(respond(update_profile(cx, &caller, &input).await?))
    })
    .await
}
