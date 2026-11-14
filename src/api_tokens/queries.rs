use sqlx::PgExecutor;

pub struct NewApiTokenRow<'a> {
    pub user_id: i32,
    pub public_id: &'a str,
    pub name: &'a str,
    pub token_hash: &'a str,
    pub last_four: &'a str,
    pub scopes: &'a [String],
    pub audience: &'a str,
    pub expires_at: time::OffsetDateTime,
    pub max_per_user: i64,
}

#[derive(Debug)]
pub struct ApiTokenSummary {
    pub public_id: String,
    pub name: String,
    pub last_four: String,
    pub scopes: Vec<String>,
    pub audience: String,
    pub created_at: time::OffsetDateTime,
    pub expires_at: time::OffsetDateTime,
    pub last_used_at: Option<time::OffsetDateTime>,
}

#[derive(Debug)]
pub struct ApiTokenGrant {
    pub id: i32,
    pub user_id: i32,
    pub scopes: Vec<String>,
    pub audience: String,
}

pub async fn query_insert_api_token(
    token: &NewApiTokenRow<'_>,
    db: impl PgExecutor<'_>,
) -> Result<Option<i32>, sqlx::Error> {
    let id = sqlx::query_scalar!(
        "INSERT INTO api_tokens (user_id, public_id, name, token_hash, last_four, scopes, audience, expires_at) SELECT $1, $2, $3, $4, $5, $6, $7, $8 WHERE (SELECT count(*) FROM api_tokens WHERE user_id = $1) < $9 RETURNING id",
        token.user_id,
        token.public_id,
        token.name,
        token.token_hash,
        token.last_four,
        token.scopes,
        token.audience,
        token.expires_at,
        token.max_per_user,
    )
    .fetch_optional(db)
    .await?;

    Ok(id)
}

pub async fn query_list_api_tokens(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Vec<ApiTokenSummary>, sqlx::Error> {
    let tokens = sqlx::query_as!(
        ApiTokenSummary,
        "SELECT public_id, name, last_four, scopes, audience, created_at, expires_at, last_used_at FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
        user_id,
    )
    .fetch_all(db)
    .await?;

    Ok(tokens)
}

pub async fn query_get_api_token_by_hash(
    token_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<ApiTokenGrant>, sqlx::Error> {
    let grant = sqlx::query_as!(
        ApiTokenGrant,
        "SELECT id, user_id, scopes, audience FROM api_tokens WHERE token_hash = $1 AND expires_at > now()",
        token_hash,
    )
    .fetch_optional(db)
    .await?;

    Ok(grant)
}

pub async fn query_record_api_token_use(
    id: i32,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE api_tokens SET last_used_at = now() WHERE id = $1 AND (last_used_at IS NULL OR last_used_at < now() - interval '5 minutes')",
        id,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_delete_api_token(
    public_id: &str,
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<String>, sqlx::Error> {
    let name = sqlx::query_scalar!(
        "DELETE FROM api_tokens WHERE public_id = $1 AND user_id = $2 RETURNING name",
        public_id,
        user_id,
    )
    .fetch_optional(db)
    .await?;

    Ok(name)
}

pub async fn query_delete_user_api_tokens(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM api_tokens WHERE user_id = $1", user_id)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

pub async fn query_delete_expired_api_tokens(
    expired_for: time::Duration,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let cutoff = time::OffsetDateTime::now_utc() - expired_for;
    let result = sqlx::query!("DELETE FROM api_tokens WHERE expires_at < $1", cutoff)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}
