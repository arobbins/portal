use sqlx::PgExecutor;

use crate::auth::links::TokenPurpose;

pub struct NewUserToken<'a> {
    pub user_id: i32,
    pub purpose: TokenPurpose,
    pub token_hash: &'a str,
    pub expires_at: time::OffsetDateTime,
}

pub async fn query_insert_user_token(
    token: &NewUserToken<'_>,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO user_tokens (user_id, purpose, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        token.user_id,
        token.purpose.as_str(),
        token.token_hash,
        token.expires_at,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_get_valid_user_token(
    purpose: TokenPurpose,
    token_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT user_id FROM user_tokens WHERE purpose = $1 AND token_hash = $2 AND expires_at > now()",
        purpose.as_str(),
        token_hash,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_consume_user_token(
    purpose: TokenPurpose,
    token_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM user_tokens WHERE purpose = $1 AND token_hash = $2 AND expires_at > now() RETURNING user_id",
        purpose.as_str(),
        token_hash,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_delete_user_tokens(
    user_id: i32,
    purpose: TokenPurpose,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM user_tokens WHERE user_id = $1 AND purpose = $2",
        user_id,
        purpose.as_str(),
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_delete_expired_user_tokens(db: impl PgExecutor<'_>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM user_tokens WHERE expires_at <= now()")
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}
