use topcoat::{
    Result,
    context::Cx,
    router::{Body, StatusCode},
};

use crate::action::ActionError;
use crate::api::action::{ApiAction, NoInput, PublicApiAction};
use crate::api::handler::{api_call, api_call_with_body, public_api_call};
use crate::api::problem::{ApiResponse, respond};
use crate::web::env;

pub async fn serve<A: ApiAction<Input = NoInput>>(cx: &Cx) -> Result<ApiResponse> {
    api_call(cx, async |caller| {
        Ok(respond(A::run(&env(cx), &caller, &NoInput {}).await?))
    })
    .await
}

pub async fn serve_with_body<A: ApiAction>(cx: &Cx, body: Body) -> Result<ApiResponse> {
    let key = idempotency_key(cx);
    api_call_with_body(cx, body, async |caller, input: A::Input| {
        let env = env(cx).with_idempotency_key(key.as_deref());
        Ok(with_success_status(
            A::SUCCESS,
            respond(A::run(&env, &caller, &input).await?),
        ))
    })
    .await
}

pub async fn serve_public<A: PublicApiAction<Input = NoInput>>(cx: &Cx) -> Result<ApiResponse> {
    public_api_call(
        async {
            let outcome = A::run_public(&env(cx), &NoInput {}).await?;
            Ok(respond(outcome.map_err(ActionError::Problem)))
        }
        .await,
    )
}

fn idempotency_key(cx: &Cx) -> Option<String> {
    topcoat::router::request::headers(cx)
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn with_success_status(success: StatusCode, response: ApiResponse) -> ApiResponse {
    match response {
        (StatusCode::OK, headers, body) => (success, headers, body),
        other => other,
    }
}
