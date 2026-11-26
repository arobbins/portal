use serde_json::{Value, json};
use topcoat::router::StatusCode;

use crate::action::ActionProblem;
use crate::api::problem::{ApiResponse, no_store};
use topcoat::router::content::Json;

pub fn oauth_json(status: StatusCode, body: Value) -> ApiResponse {
    (status, no_store(), Json(body))
}

pub fn oauth_error(problem: impl ActionProblem) -> ApiResponse {
    oauth_json(
        problem.spec().status,
        json!({ "error": problem.code(), "error_description": problem.message() }),
    )
}
