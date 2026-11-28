use sqlx::PgExecutor;

use crate::api::scope::ApiScope;
use time::OffsetDateTime;

pub struct NewConnection<'a> {
    pub public_id: &'a str,
    pub user_id: i32,
    pub client_id: i32,
    pub return_host: &'a str,
    pub scopes: &'a [ApiScope],
    pub max_per_user: i64,
    pub now: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredConnection {
    pub id: i32,
    pub public_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionRow {
    pub public_id: String,
    pub name: String,
    pub return_host: String,
    pub scopes: Vec<ApiScope>,
    pub created_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedConnection {
    pub user_id: i32,
    pub public_id: String,
    pub name: String,
    pub return_host: String,
}

pub async fn query_upsert_connection(
    connection: &NewConnection<'_>,
    db: impl PgExecutor<'_>,
) -> Result<Option<StoredConnection>, sqlx::Error> {
    sqlx::query_as!(
        StoredConnection,
        "INSERT INTO oauth_connections (public_id, user_id, client_id, return_host, scopes, created_at, updated_at) SELECT $1, $2, $3, $4, $5, $7, $7 WHERE (SELECT count(*) FROM oauth_connections WHERE user_id = $2) < $6 OR EXISTS (SELECT 1 FROM oauth_connections WHERE user_id = $2 AND client_id = $3) ON CONFLICT (user_id, client_id) DO UPDATE SET scopes = EXCLUDED.scopes, return_host = EXCLUDED.return_host, updated_at = EXCLUDED.updated_at RETURNING id, public_id",
        connection.public_id,
        connection.user_id,
        connection.client_id,
        connection.return_host,
        connection.scopes as &[ApiScope],
        connection.max_per_user,
        connection.now,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_get_connection_scopes(
    id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<Vec<ApiScope>>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT scopes AS "scopes: Vec<ApiScope>" FROM oauth_connections WHERE id = $1"#,
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn query_list_connections(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Vec<ConnectionRow>, sqlx::Error> {
    sqlx::query_as!(
        ConnectionRow,
        r#"SELECT o.public_id, c.name, o.return_host, o.scopes AS "scopes: Vec<ApiScope>", o.created_at, o.last_used_at FROM oauth_connections o JOIN oauth_clients c ON c.id = o.client_id WHERE o.user_id = $1 ORDER BY o.created_at DESC, o.id DESC"#,
        user_id,
    )
    .fetch_all(db)
    .await
}

pub async fn query_delete_connection(
    public_id: &str,
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<EndedConnection>, sqlx::Error> {
    sqlx::query_as!(
        EndedConnection,
        "WITH ended AS (DELETE FROM oauth_connections WHERE public_id = $1 AND user_id = $2 RETURNING user_id, public_id, client_id, return_host) SELECT ended.user_id, ended.public_id, c.name, ended.return_host FROM ended JOIN oauth_clients c ON c.id = ended.client_id",
        public_id,
        user_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_delete_connection_by_id(
    id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<EndedConnection>, sqlx::Error> {
    sqlx::query_as!(
        EndedConnection,
        "WITH ended AS (DELETE FROM oauth_connections WHERE id = $1 RETURNING user_id, public_id, client_id, return_host) SELECT ended.user_id, ended.public_id, c.name, ended.return_host FROM ended JOIN oauth_clients c ON c.id = ended.client_id",
        id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_delete_user_connections(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM oauth_connections WHERE user_id = $1", user_id)
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}

pub async fn query_record_connection_use(
    id: i32,
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE oauth_connections SET last_used_at = $2 WHERE id = $1 AND (last_used_at IS NULL OR last_used_at < $2::timestamptz - interval '5 minutes')",
        id,
        now,
    )
    .execute(db)
    .await?;
    Ok(())
}
