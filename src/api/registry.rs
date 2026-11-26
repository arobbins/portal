use std::future::Future;
use std::pin::Pin;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use topcoat::{
    Result,
    context::Cx,
    router::{Body, StatusCode, content::Json},
};

use crate::action::{ActionError, ActionProblem, ActionResult, Caller, Outcome};
use crate::api::body::BodyProblem;
use crate::api::catalog::{Endpoint, ErrorDoc, HttpMethod};
use crate::api::handler::{api_call, api_call_with_body, public_api_call};
use crate::api::problem::{ApiFailure, ApiResponse, action_failed, respond};
use crate::billing::api::{GetBillingAction, ListPlansAction, StartCheckoutAction};
use crate::env::Env;
use crate::feedback::api::SendFeedbackAction;
use crate::permission::{Permission, TokenAccess};
use crate::preferences::api::{GetNewslettersAction, UpdateNewslettersAction};
use crate::profile::api::{GetProfileAction, UpdateProfileAction};
use crate::web::env;

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoInput {}

pub trait ApiAction {
    const ID: &'static str;
    const GROUP: &'static str;
    const METHOD: HttpMethod;
    const PATH: &'static str;
    const SUMMARY: &'static str;
    const DESCRIPTION: &'static str;
    const PERMISSION: Permission;
    const SUCCESS: StatusCode = StatusCode::OK;

    type Input: DeserializeOwned + Serialize + JsonSchema + Send + Sync;
    type Output: Serialize + JsonSchema + Send;
    type Problem: ActionProblem;

    fn run<'a>(
        env: &'a Env<'a>,
        caller: &'a Caller,
        input: &'a Self::Input,
    ) -> impl Future<Output = ActionResult<Self::Output, Self::Problem>> + Send + 'a;

    fn example_input() -> Option<Self::Input>;
    fn example_output() -> Self::Output;
    fn problems() -> Vec<ErrorDoc>;
}

pub trait PublicApiAction: ApiAction {
    fn run_public<'a>(
        env: &'a Env<'a>,
        input: &'a Self::Input,
    ) -> impl Future<Output = Outcome<Self::Output, Self::Problem>> + Send + 'a;
}

pub fn tools() -> Vec<Tool> {
    vec![
        tool::<GetProfileAction>(),
        tool::<UpdateProfileAction>(),
        tool::<GetNewslettersAction>(),
        tool::<UpdateNewslettersAction>(),
        tool::<SendFeedbackAction>(),
        public_tool::<ListPlansAction>(),
        tool::<GetBillingAction>(),
        tool::<StartCheckoutAction>(),
    ]
}

pub fn feedback_tool() -> Tool {
    tool::<SendFeedbackAction>()
}

pub fn endpoints() -> Vec<Endpoint> {
    tools().into_iter().map(|tool| tool.endpoint).collect()
}

pub enum ToolOutcome {
    Success(Value),
    InvalidArguments(String),
    Failed(Value),
}

type ToolFuture<'a> = Pin<Box<dyn Future<Output = Result<ToolOutcome>> + Send + 'a>>;

type PublicRun = for<'a> fn(&'a Env<'a>, Value) -> ToolFuture<'a>;

pub struct Tool {
    pub endpoint: Endpoint,
    pub permission: Permission,
    pub run: for<'a> fn(&'a Env<'a>, &'a Caller, Value) -> ToolFuture<'a>,
    pub run_public: Option<PublicRun>,
}

impl Tool {
    pub fn name(&self) -> String {
        self.endpoint.id.replace('-', "_")
    }
}

fn tool<A: ApiAction>() -> Tool {
    Tool {
        endpoint: endpoint::<A>(),
        permission: A::PERMISSION,
        run: run_tool::<A>,
        run_public: None,
    }
}

fn public_tool<A: PublicApiAction>() -> Tool {
    Tool {
        run_public: Some(run_public_tool::<A>),
        ..tool::<A>()
    }
}

fn run_public_tool<'a, A: PublicApiAction>(env: &'a Env<'a>, arguments: Value) -> ToolFuture<'a> {
    Box::pin(async move {
        let input: A::Input = match serde_json::from_value(arguments) {
            Ok(input) => input,
            Err(e) => return Ok(ToolOutcome::InvalidArguments(e.to_string())),
        };
        Ok(tool_outcome(
            A::run_public(env, &input)
                .await?
                .map_err(ActionError::Problem),
        ))
    })
}

fn tool_outcome<T: Serialize, P: ActionProblem>(outcome: Result<T, ActionError<P>>) -> ToolOutcome {
    match outcome {
        Ok(output) => ToolOutcome::Success(to_value(&output)),
        Err(error) => {
            let (_, _, Json(body)) = action_failed(error);
            ToolOutcome::Failed(body)
        }
    }
}

fn run_tool<'a, A: ApiAction>(
    env: &'a Env<'a>,
    caller: &'a Caller,
    arguments: Value,
) -> ToolFuture<'a> {
    Box::pin(async move {
        let input: A::Input = match serde_json::from_value(arguments) {
            Ok(input) => input,
            Err(e) => return Ok(ToolOutcome::InvalidArguments(e.to_string())),
        };
        Ok(tool_outcome(A::run(env, caller, &input).await?))
    })
}

fn has_body(method: HttpMethod) -> bool {
    method != HttpMethod::Get
}

pub fn endpoint<A: ApiAction>() -> Endpoint {
    let body = has_body(A::METHOD);
    let public = A::PERMISSION.token_access() == TokenAccess::Public;
    let mut errors = Vec::new();
    if !public {
        errors.push(ErrorDoc::of(ApiFailure::InsufficientScope));
    }
    if body {
        errors.extend(BodyProblem::ALL.into_iter().map(ErrorDoc::of));
    }
    errors.extend(A::problems());

    Endpoint {
        id: A::ID,
        group: A::GROUP,
        method: A::METHOD,
        path: A::PATH,
        summary: A::SUMMARY,
        description: A::DESCRIPTION,
        public,
        success: A::SUCCESS.as_u16(),
        scope: match A::PERMISSION.token_access() {
            TokenAccess::Scope(scope) => Some(scope),
            TokenAccess::AnyToken | TokenAccess::Public => None,
            TokenAccess::BrowserOnly => panic!("an ApiAction's permission must allow tokens"),
        },
        request: body.then(|| schema_for!(A::Input)),
        response: schema_for!(A::Output),
        example_request: body
            .then(A::example_input)
            .flatten()
            .map(|input| to_value(&input)),
        example_response: to_value(&A::example_output()),
        errors,
    }
}

fn to_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_ids_are_unique_and_each_method_and_path_is_used_once() {
        let endpoints = endpoints();
        for endpoint in &endpoints {
            assert_eq!(
                endpoints
                    .iter()
                    .filter(|other| other.id == endpoint.id)
                    .count(),
                1,
                "{}",
                endpoint.id
            );
            assert_eq!(
                endpoints
                    .iter()
                    .filter(|other| other.method == endpoint.method && other.path == endpoint.path)
                    .count(),
                1,
                "{} {}",
                endpoint.method.as_str(),
                endpoint.path
            );
            assert!(endpoint.path.starts_with("/api/v1/"));
        }
    }

    #[test]
    fn only_requests_with_a_body_document_one() {
        for endpoint in endpoints() {
            assert_eq!(endpoint.request.is_some(), has_body(endpoint.method));
            assert_eq!(
                endpoint.example_request.is_some(),
                has_body(endpoint.method)
            );
        }
    }
}
