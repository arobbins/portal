use sqlx::PgExecutor;

use crate::api::scope::ApiScope;
use time::OffsetDateTime;

pub struct NewRequest<'a> {
    pub public_id: &'a str,
    pub client_id: i32,
    pub redirect_uri: &'a str,
    pub scopes: &'a [ApiScope],
    pub state: Option<&'a str>,
    pub code_challenge: &'a str,
    pub now: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum RequestStatus {
    Pending,
    Approved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestRow {
    pub id: i32,
    pub public_id: String,
    pub client_id: i32,
    pub client_name: String,
    pub redirect_uri: String,
    pub scopes: Vec<ApiScope>,
    pub state: Option<String>,
    pub status: RequestStatus,
    pub acted_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedCode {
    pub connection_id: i32,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub acted_at: OffsetDateTime,
}

pub async fn query_insert_request(
    request: &NewRequest<'_>,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO oauth_requests (public_id, client_id, redirect_uri, scopes, state, code_challenge, acted_at, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
        request.public_id,
        request.client_id,
        request.redirect_uri,
        request.scopes as &[ApiScope],
        request.state,
        request.code_challenge,
        request.now,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn query_get_request(
    public_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<RequestRow>, sqlx::Error> {
    sqlx::query_as!(
        RequestRow,
        "SELECT r.id, r.public_id, r.client_id, c.name AS client_name, r.redirect_uri, r.scopes AS \"scopes: Vec<ApiScope>\", r.state, r.status AS \"status: RequestStatus\", r.acted_at FROM oauth_requests r JOIN oauth_clients c ON c.id = r.client_id WHERE r.public_id = $1",
        public_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_touch_request(
    id: i32,
    now: OffsetDateTime,
    cutoff: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE oauth_requests SET acted_at = $2 WHERE id = $1 AND status = 'pending' AND acted_at > $3",
        id,
        now,
        cutoff,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn query_approve_request(
    id: i32,
    user_id: i32,
    connection_id: i32,
    code_hash: &str,
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE oauth_requests SET status = 'approved', user_id = $2, connection_id = $3, code_hash = $4, acted_at = $5 WHERE id = $1 AND status = 'pending'",
        id,
        user_id,
        connection_id,
        code_hash,
        now,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn query_delete_approved_requests(
    connection_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM oauth_requests WHERE connection_id = $1 AND status = 'approved'",
        connection_id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn query_delete_pending_request(
    id: i32,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM oauth_requests WHERE id = $1 AND status = 'pending'",
        id
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn query_consume_code(
    code_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<IssuedCode>, sqlx::Error> {
    sqlx::query_as!(
        IssuedCode,
        r#"WITH used AS (DELETE FROM oauth_requests WHERE code_hash = $1 AND status = 'approved' RETURNING connection_id, client_id, redirect_uri, code_challenge, acted_at) SELECT used.connection_id AS "connection_id!", c.client_id, used.redirect_uri, used.code_challenge, used.acted_at FROM used JOIN oauth_clients c ON c.id = used.client_id"#,
        code_hash,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_delete_stale_requests(
    cutoff: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM oauth_requests WHERE acted_at < $1", cutoff)
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}
