use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Deserialize;
use topcoat::router::StatusCode;

use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::api::scope::{ApiScope, parse_scopes};
use crate::env::Env;
use crate::oauth::lifecycle::record_disconnected;
use crate::oauth::queries::{query_delete_connection, query_list_connections};
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::events::EventSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectProblem {
    NotFound,
}

impl ActionProblem for DisconnectProblem {
    fn spec(self) -> ProblemSpec {
        ProblemSpec::new("app_not_found", "That connected app doesn't exist.")
            .with_status(StatusCode::NOT_FOUND)
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Disconnect an app that signed in to the account.")]
pub struct DisconnectApp {
    #[schemars(description = "The connection's id (app_…).")]
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub return_host: String,
    pub scopes: Vec<ApiScope>,
    pub connected_at: time::OffsetDateTime,
    pub last_used_at: Option<time::OffsetDateTime>,
}

pub async fn list_connected_apps(
    env: &Env<'_>,
    caller: &Caller,
) -> ActionResult<Vec<Connection>, Infallible> {
    authorize!(caller.require(Permission::ManageConnectedApps));
    let rows = query_list_connections(caller.owner_id(), env.db()).await?;

    Ok(Ok(rows
        .into_iter()
        .map(|row| Connection {
            id: row.public_id,
            name: row.name,
            return_host: row.return_host,
            scopes: parse_scopes(&row.scopes),
            connected_at: row.created_at,
            last_used_at: row.last_used_at,
        })
        .collect()))
}

pub async fn disconnect_app(
    env: &Env<'_>,
    caller: &Caller,
    input: &DisconnectApp,
) -> ActionResult<(), DisconnectProblem> {
    authorize!(caller.require(Permission::ManageConnectedApps));
    if !PublicIdKind::ConnectedApp.is_valid(&input.id) {
        return fail(DisconnectProblem::NotFound);
    }
    let mut tx = env.db().begin().await?;
    let Some(ended) = query_delete_connection(&input.id, caller.owner_id(), &mut *tx).await? else {
        return fail(DisconnectProblem::NotFound);
    };
    record_disconnected(env, &EventSource::of(caller), &ended, &mut tx).await?;
    tx.commit().await?;

    Ok(Ok(()))
}
