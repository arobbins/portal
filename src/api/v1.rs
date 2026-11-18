use topcoat::{Result, context::Cx, router::route};

use crate::api::handler::public_api_call;
use crate::api::problem::{ApiFailure, ApiProblem, ApiResponse};

pub mod docs;
pub mod feedback;
pub mod profile;

#[route(* "/api/{*rest}")]
pub async fn unknown_api_route(cx: &Cx) -> Result<ApiResponse> {
    public_api_call(Ok(ApiProblem::of(ApiFailure::NotFound).response()))
}
