use std::sync::Arc;

use serde_json::json;
use sqlx::{PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;

use crate::action::{Caller, ConnectedApp};
use crate::api::scope::parse_scopes;
use crate::env::{App, Env};
use crate::oauth::app_label;
use crate::oauth::queries::{
    EndedConnection, query_delete_expired_tokens, query_delete_stale_requests,
    query_delete_unapproved_clients, query_delete_user_connections, query_get_access_grant,
    query_record_connection_use,
};
use crate::oauth::secrets::{REQUEST_IDLE_LIMIT, SecretKind};
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::AccountStatus;
use crate::user::queries::query_get_user_by_id;

const UNAPPROVED_CLIENT_LIFETIME: time::Duration = time::Duration::hours(24);

pub fn is_app_token(raw: &str) -> bool {
    SecretKind::AccessToken.hash_of(raw).is_some()
}

pub async fn app_caller(
    raw: &str,
    now: OffsetDateTime,
    db: &PgPool,
) -> Result<Option<Caller>, sqlx::Error> {
    let Some(hash) = SecretKind::AccessToken.hash_of(raw) else {
        return Ok(None);
    };
    let Some(grant) = query_get_access_grant(&hash, now, db).await? else {
        return Ok(None);
    };
    let Some(user) = query_get_user_by_id(grant.user_id, db).await? else {
        return Ok(None);
    };
    if !matches!(user.status(), AccountStatus::Active { .. }) {
        return Ok(None);
    }
    let scopes = parse_scopes(&grant.scopes)
        .into_iter()
        .filter(|scope| scope.available_to(&user) && scope.offered_by_sign_in())
        .collect();
    let app = ConnectedApp {
        connection_id: grant.connection_id,
        scopes,
        label: app_label(&grant.name, &grant.return_host),
    };
    Ok(Some(Caller::connected_app(Arc::new(user), app)))
}

pub async fn record_app_use(
    connection_id: i32,
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    query_record_connection_use(connection_id, now, db).await
}

pub async fn record_disconnected(
    env: &Env<'_>,
    source: &EventSource,
    ended: &EndedConnection,
    tx: &mut PgConnection,
) -> topcoat::Result<()> {
    let detail = json!({
        "connection": ended.public_id,
        "name": app_label(&ended.name, &ended.return_host),
    });
    let kind = AccountEventKind::AppDisconnected;
    record_event(env, source, kind, detail, &mut *tx).await
}

pub async fn disconnect_all_for_user(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    query_delete_user_connections(user_id, db).await
}

pub async fn delete_expired(app: &App) -> Result<u64, sqlx::Error> {
    let now = app.clock.now();
    let tokens = query_delete_expired_tokens(now, &app.db).await?;
    let requests = query_delete_stale_requests(now - REQUEST_IDLE_LIMIT, &app.db).await?;
    let clients =
        query_delete_unapproved_clients(now - UNAPPROVED_CLIENT_LIFETIME, &app.db).await?;
    Ok(tokens + requests + clients)
}

#[cfg(test)]
mod tests;
