use serde_json::json;
use sqlx::PgConnection;

use crate::env::Env;
use crate::oauth::app_label;
use crate::oauth::queries::{
    EndedConnection, query_delete_connection_by_id, query_get_token_owner,
};
use crate::oauth::secrets::SecretKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};

pub async fn end_connection(env: &Env<'_>, connection_id: i32) -> topcoat::Result<()> {
    let mut tx = env.db().begin().await?;
    if let Some(ended) = query_delete_connection_by_id(connection_id, &mut *tx).await? {
        let label = app_label(&ended.name, &ended.return_host);
        let source = EventSource::app(ended.user_id, label);
        record_disconnected(env, &source, &ended, &mut tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn revoke_token(env: &Env<'_>, token: &str, client_id: &str) -> topcoat::Result<()> {
    let hash = SecretKind::AccessToken
        .hash_of(token)
        .or_else(|| SecretKind::RefreshToken.hash_of(token));
    let Some(hash) = hash else {
        return Ok(());
    };
    match query_get_token_owner(&hash, env.db()).await? {
        Some(owner) if owner.client_id == client_id => {
            end_connection(env, owner.connection_id).await
        }
        _ => Ok(()),
    }
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
