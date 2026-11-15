use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use topcoat::router::StatusCode;

use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::api_tokens::queries::{ApiToken, query_delete_api_token, query_list_api_tokens};
use crate::env::Env;
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokeTokenProblem {
    NotFound,
}

impl ActionProblem for RevokeTokenProblem {
    fn spec(self) -> ProblemSpec {
        ProblemSpec::new("token_not_found", "That API token doesn't exist.")
            .with_status(StatusCode::NOT_FOUND)
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Revoke an API token.")]
pub struct RevokeToken {
    #[schemars(description = "The token's id (tok_…).")]
    pub id: String,
}

pub async fn list_api_tokens(
    env: &Env<'_>,
    caller: &Caller,
) -> ActionResult<Vec<ApiToken>, Infallible> {
    authorize!(caller.require(Permission::ManageApiTokens));

    Ok(Ok(query_list_api_tokens(caller.owner_id(), env.db()).await?))
}

pub async fn revoke_api_token(
    env: &Env<'_>,
    caller: &Caller,
    input: &RevokeToken,
) -> ActionResult<String, RevokeTokenProblem> {
    authorize!(caller.require(Permission::ManageApiTokens));
    if !PublicIdKind::ApiToken.is_valid(&input.id) {
        return fail(RevokeTokenProblem::NotFound);
    }
    let mut tx = env.db().begin().await?;
    let Some(name) = query_delete_api_token(&input.id, caller.owner_id(), &mut *tx).await? else {
        return fail(RevokeTokenProblem::NotFound);
    };
    let detail = json!({ "token": input.id, "name": name });
    record_event(
        env,
        &EventSource::of(caller),
        AccountEventKind::ApiTokenRevoked,
        detail,
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(name))
}
