use sqlx::PgExecutor;

use crate::env::App;
use crate::oauth::queries::{
    query_delete_expired_tokens, query_delete_stale_requests, query_delete_unapproved_clients,
    query_delete_user_connections,
};
use crate::oauth::secrets::REQUEST_IDLE_LIMIT;

const UNAPPROVED_CLIENT_LIFETIME: time::Duration = time::Duration::hours(24);

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
