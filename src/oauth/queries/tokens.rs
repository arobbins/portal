use sqlx::PgExecutor;
use time::OffsetDateTime;

pub struct NewToken<'a> {
    pub connection_id: i32,
    pub kind: &'a str,
    pub token_hash: &'a str,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppGrant {
    pub connection_id: i32,
    pub user_id: i32,
    pub scopes: Vec<String>,
    pub name: String,
    pub return_host: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshRow {
    pub id: i32,
    pub connection_id: i32,
    pub user_id: i32,
    pub client_id: String,
    pub replaced_at: Option<OffsetDateTime>,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenOwner {
    pub connection_id: i32,
    pub client_id: String,
}

pub async fn query_insert_token(
    token: &NewToken<'_>,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO oauth_tokens (connection_id, kind, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        token.connection_id,
        token.kind,
        token.token_hash,
        token.expires_at,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn query_get_access_grant(
    token_hash: &str,
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<Option<AppGrant>, sqlx::Error> {
    sqlx::query_as!(
        AppGrant,
        "SELECT o.id AS connection_id, o.user_id, o.scopes, c.name, o.return_host FROM oauth_tokens t JOIN oauth_connections o ON o.id = t.connection_id JOIN oauth_clients c ON c.id = o.client_id WHERE t.token_hash = $1 AND t.kind = 'access' AND t.expires_at > $2",
        token_hash,
        now,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_get_refresh_token(
    token_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<RefreshRow>, sqlx::Error> {
    sqlx::query_as!(
        RefreshRow,
        "SELECT t.id, t.connection_id, o.user_id, c.client_id, t.replaced_at, t.expires_at FROM oauth_tokens t JOIN oauth_connections o ON o.id = t.connection_id JOIN oauth_clients c ON c.id = o.client_id WHERE t.token_hash = $1 AND t.kind = 'refresh'",
        token_hash,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_replace_refresh_token(
    id: i32,
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE oauth_tokens SET replaced_at = $2 WHERE id = $1 AND kind = 'refresh' AND replaced_at IS NULL",
        id,
        now,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn query_delete_access_tokens(
    connection_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM oauth_tokens WHERE connection_id = $1 AND kind = 'access'",
        connection_id,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn query_delete_connection_tokens(
    connection_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM oauth_tokens WHERE connection_id = $1",
        connection_id,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn query_get_token_owner(
    token_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<TokenOwner>, sqlx::Error> {
    sqlx::query_as!(
        TokenOwner,
        "SELECT t.connection_id, c.client_id FROM oauth_tokens t JOIN oauth_connections o ON o.id = t.connection_id JOIN oauth_clients c ON c.id = o.client_id WHERE t.token_hash = $1",
        token_hash,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_delete_expired_tokens(
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM oauth_tokens WHERE expires_at < $1", now)
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}
