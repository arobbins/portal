use std::future::Future;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use topcoat::router::StatusCode;

use crate::action::{ActionResult, Caller, ListedProblem, Outcome};
use crate::api::catalog::HttpMethod;
use crate::env::Env;
use crate::permission::Permission;

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
    type Problem: ListedProblem;

    fn run<'a>(
        env: &'a Env<'a>,
        caller: &'a Caller,
        input: &'a Self::Input,
    ) -> impl Future<Output = ActionResult<Self::Output, Self::Problem>> + Send + 'a;

    fn example_input() -> Option<Self::Input>;
    fn example_output() -> Self::Output;
}

pub trait PublicApiAction: ApiAction {
    fn run_public<'a>(
        env: &'a Env<'a>,
        input: &'a Self::Input,
    ) -> impl Future<Output = Outcome<Self::Output, Self::Problem>> + Send + 'a;
}
