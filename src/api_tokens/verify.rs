use sqlx::PgExecutor;

use crate::api::scope::TokenAudience;
use crate::api_tokens::queries::{
    ApiTokenGrant, query_delete_expired_api_tokens, query_delete_user_api_tokens,
    query_get_api_token_by_hash, query_record_api_token_use,
};
use crate::api_tokens::token::is_well_formed;
use crate::secret::hash_secret;

const EXPIRED_TOKEN_GRACE: time::Duration = time::Duration::days(7);

pub async fn find_grant(
    raw: &str,
    audience: TokenAudience,
    db: impl PgExecutor<'_>,
) -> Result<Option<ApiTokenGrant>, sqlx::Error> {
    if !is_well_formed(raw) {
        return Ok(None);
    }
    let grant = query_get_api_token_by_hash(&hash_secret(raw), db).await?;

    Ok(grant.filter(|grant| grant.audience == audience.as_str()))
}

pub async fn record_use(token_id: i32, db: impl PgExecutor<'_>) -> Result<(), sqlx::Error> {
    query_record_api_token_use(token_id, db).await
}

pub async fn revoke_all_for_user(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    query_delete_user_api_tokens(user_id, db).await
}

pub async fn delete_long_expired(db: impl PgExecutor<'_>) -> Result<u64, sqlx::Error> {
    query_delete_expired_api_tokens(EXPIRED_TOKEN_GRACE, db).await
}
