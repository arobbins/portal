use topcoat::{Result, context::Cx, router::route};

use crate::api::catalog::openapi;
use crate::api::handler::public_api_call;
use crate::api::problem::{ApiResponse, ok};
use crate::web::config;

#[route(GET "/api/v1/openapi.json")]
pub async fn openapi_document(cx: &Cx) -> Result<ApiResponse> {
    public_api_call(Ok(ok(&openapi(config(cx).origin()))))
}
