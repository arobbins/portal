use std::future::Future;
use std::pin::Pin;

use serde::Serialize;
use serde_json::Value;
use topcoat::{Result, router::content::Json};

use crate::action::{ActionError, ActionProblem, Caller};
use crate::api::action::{ApiAction, PublicApiAction};
use crate::api::catalog::Endpoint;
use crate::api::problem::action_failed;
use crate::api::registry::{endpoint, to_value};
use crate::env::Env;
use crate::permission::Permission;

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

pub fn tool<A: ApiAction>() -> Tool {
    Tool {
        endpoint: endpoint::<A>(),
        permission: A::PERMISSION,
        run: run_tool::<A>,
        run_public: None,
    }
}

pub fn public_tool<A: PublicApiAction>() -> Tool {
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
