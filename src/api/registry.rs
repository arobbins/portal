use std::future::Future;
use std::pin::Pin;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use topcoat::{
    Result,
    context::Cx,
    router::{Body, content::Json},
};

use crate::action::{ActionProblem, ActionResult, Caller};
use crate::api::body::BodyProblem;
use crate::api::catalog::{Endpoint, ErrorDoc, HttpMethod};
use crate::api::handler::{api_call, api_call_with_body};
use crate::api::problem::{ApiFailure, ApiResponse, action_failed, respond};
use crate::env::Env;
use crate::feedback::api::SendFeedbackAction;
use crate::permission::{Permission, TokenAccess};
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

pub fn tools() -> Vec<Tool> {
    vec![
        tool::<GetProfileAction>(),
        tool::<UpdateProfileAction>(),
        tool::<SendFeedbackAction>(),
    ]
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

pub struct Tool {
    pub endpoint: Endpoint,
    pub permission: Permission,
    pub run: for<'a> fn(&'a Env<'a>, &'a Caller, Value) -> ToolFuture<'a>,
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
        Ok(match A::run(env, caller, &input).await? {
            Ok(output) => ToolOutcome::Success(to_value(&output)),
            Err(error) => {
                let (_, _, Json(body)) = action_failed(error);
                ToolOutcome::Failed(body)
            }
        })
    })
}

fn has_body(method: HttpMethod) -> bool {
    method != HttpMethod::Get
}

pub fn endpoint<A: ApiAction>() -> Endpoint {
    let body = has_body(A::METHOD);
    let mut errors = vec![ErrorDoc::of(ApiFailure::InsufficientScope)];
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
        scope: match A::PERMISSION.token_access() {
            TokenAccess::Scope(scope) => Some(scope),
            TokenAccess::AnyToken => None,
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
    api_call_with_body(cx, body, async |caller, input: A::Input| {
        Ok(respond(A::run(&env(cx), &caller, &input).await?))
    })
    .await
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
