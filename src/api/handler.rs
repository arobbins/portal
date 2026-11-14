use serde::de::DeserializeOwned;
use topcoat::{Result, context::Cx, router::Body};

use crate::action::Caller;
use crate::api::auth::authenticate;
use crate::api::body::read_json;
use crate::api::problem::{ApiResponse, or_internal_error};
use crate::api::scope::TokenAudience;

pub async fn api_call(
    cx: &Cx,
    run: impl AsyncFnOnce(Caller) -> Result<ApiResponse>,
) -> Result<ApiResponse> {
    Ok(or_internal_error(
        async {
            let caller = match authenticate(cx, TokenAudience::Api).await? {
                Ok(caller) => caller,
                Err(problem) => return Ok(problem.response()),
            };
            run(caller).await
        }
        .await,
    ))
}

pub async fn api_call_with_body<T: DeserializeOwned>(
    cx: &Cx,
    body: Body,
    run: impl AsyncFnOnce(Caller, T) -> Result<ApiResponse>,
) -> Result<ApiResponse> {
    api_call(cx, async |caller| match read_json::<T>(cx, body).await? {
        Ok(input) => run(caller, input).await,
        Err(response) => Ok(response),
    })
    .await
}

pub fn public_api_call(result: Result<ApiResponse>) -> Result<ApiResponse> {
    Ok(or_internal_error(result))
}
