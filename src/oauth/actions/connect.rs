use serde_json::json;
use sqlx::PgConnection;
use topcoat::router::StatusCode;

use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::api::scope::ApiScope;
use crate::auth::reauth::{PasskeyProof, ReauthProblem, check_reauth};
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::oauth::authorize::{error_redirect, issuer, live_request, requested_scopes};
use crate::oauth::emails::app_connected_email;
use crate::oauth::queries::{
    NewConnection, RequestRow, StoredConnection, query_approve_request,
    query_delete_pending_request, query_record_client_approval, query_upsert_connection,
};
use crate::oauth::redirect::{return_host, with_params};
use crate::oauth::secrets::SecretKind;
use crate::oauth::{MAX_CONNECTIONS_PER_USER, app_label};
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectProblem {
    Expired,
    Reauth(ReauthProblem),
    TooManyApps,
}

impl ActionProblem for ConnectProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Expired => ProblemSpec::new(
                "request_expired",
                "This request has expired. Go back to the app and connect again.",
            )
            .with_status(StatusCode::BAD_REQUEST),
            Self::Reauth(problem) => problem.spec(),
            Self::TooManyApps => ProblemSpec::new(
                "too_many_apps",
                "You have too many connected apps. Disconnect one you no longer use.",
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectApp {
    pub request: String,
    pub scopes: Vec<ApiScope>,
    pub current_password: String,
}

pub async fn connect_app(
    env: &Env<'_>,
    caller: &Caller,
    input: &ConnectApp,
    passkey: Option<PasskeyProof>,
) -> ActionResult<String, ConnectProblem> {
    authorize!(caller.require(Permission::ManageConnectedApps));
    let Some(request) = live_request(env, &input.request).await? else {
        return fail(ConnectProblem::Expired);
    };
    let granted = granted_scopes(&request, &input.scopes);
    if granted.iter().any(|scope| scope.can_write()) {
        let password = &input.current_password;
        if let Err(problem) = check_reauth(env, &caller.user, password, passkey).await? {
            return fail(ConnectProblem::Reauth(problem));
        }
    }

    let code = SecretKind::Code.generate();
    let mut tx = env.db().begin().await?;
    let Some(connection) = store_connection(env, caller, &request, &granted, &mut tx).await? else {
        return fail(ConnectProblem::TooManyApps);
    };
    let owner = caller.owner_id();
    let now = env.now();
    if !query_approve_request(request.id, owner, connection.id, &code.hash, now, &mut *tx).await? {
        return fail(ConnectProblem::Expired);
    }
    query_record_client_approval(request.client_id, now, &mut *tx).await?;
    record_connected(env, caller, &request, &connection, &granted, &mut tx).await?;
    tx.commit().await?;

    let issuer = issuer(env);
    let mut params = vec![("code", code.raw.as_str()), ("iss", issuer.as_str())];
    if let Some(state) = request.state.as_deref() {
        params.push(("state", state));
    }
    Ok(Ok(with_params(&request.redirect_uri, &params)))
}

pub async fn deny_app(
    env: &Env<'_>,
    caller: &Caller,
    request_id: &str,
) -> ActionResult<String, ConnectProblem> {
    authorize!(caller.require(Permission::ManageConnectedApps));
    let Some(request) = live_request(env, request_id).await? else {
        return fail(ConnectProblem::Expired);
    };
    if !query_delete_pending_request(request.id, env.db()).await? {
        return fail(ConnectProblem::Expired);
    }
    let state = request.state.as_deref();
    Ok(Ok(error_redirect(
        env,
        &request.redirect_uri,
        "access_denied",
        state,
    )))
}

fn granted_scopes(request: &RequestRow, ticked: &[ApiScope]) -> Vec<ApiScope> {
    requested_scopes(&request.scopes.join(" "))
        .into_iter()
        .filter(|scope| ticked.contains(scope))
        .collect()
}

async fn store_connection(
    env: &Env<'_>,
    caller: &Caller,
    request: &RequestRow,
    granted: &[ApiScope],
    tx: &mut PgConnection,
) -> Result<Option<StoredConnection>, sqlx::Error> {
    let public_id = PublicIdKind::ConnectedApp.generate();
    let scopes: Vec<String> = granted
        .iter()
        .map(|scope| scope.as_str().to_string())
        .collect();
    let host = return_host(&request.redirect_uri);
    let row = NewConnection {
        public_id: &public_id,
        user_id: caller.owner_id(),
        client_id: request.client_id,
        return_host: &host,
        scopes: &scopes,
        max_per_user: MAX_CONNECTIONS_PER_USER,
        now: env.now(),
    };
    query_upsert_connection(&row, &mut *tx).await
}

async fn record_connected(
    env: &Env<'_>,
    caller: &Caller,
    request: &RequestRow,
    connection: &StoredConnection,
    granted: &[ApiScope],
    tx: &mut PgConnection,
) -> topcoat::Result<()> {
    let host = return_host(&request.redirect_uri);
    let label = app_label(&request.client_name, &host);
    let scopes: Vec<&str> = granted.iter().map(|scope| scope.as_str()).collect();
    let detail = json!({ "connection": connection.public_id, "name": label, "scopes": scopes });
    let kind = AccountEventKind::AppConnected;
    record_event(env, &EventSource::of(caller), kind, detail, &mut *tx).await?;
    let user = &caller.user;
    if env.limits().allow_email_to(&user.email).await? {
        let email = app_connected_email(env, &user.email, &request.client_name, &host, granted);
        enqueue(&SendEmail(email), &mut *tx).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
