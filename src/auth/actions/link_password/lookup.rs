use crate::auth::links::TokenPurpose;
use crate::auth::queries::{query_get_valid_user_token, query_get_valid_user_token_payload};
use crate::env::Env;
use crate::secret::hash_secret;
use crate::two_factor::actions::needs_second_factor;
use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;

pub async fn link_is_valid(
    env: &Env<'_>,
    purpose: TokenPurpose,
    token: &str,
) -> topcoat::Result<bool> {
    let user_id = query_get_valid_user_token(purpose, &hash_secret(token), env.db()).await?;
    Ok(user_id.is_some())
}

pub async fn link_payload(
    env: &Env<'_>,
    purpose: TokenPurpose,
    token: &str,
) -> topcoat::Result<Option<String>> {
    let found = query_get_valid_user_token_payload(purpose, &hash_secret(token), env.db()).await?;
    Ok(found.map(|token| token.payload))
}

pub async fn link_needs_code(
    env: &Env<'_>,
    purpose: TokenPurpose,
    token: &str,
) -> topcoat::Result<bool> {
    match link_holder(env, purpose, token).await? {
        Some(holder) => Ok(asks_for_code(env, purpose, &holder).await?),
        None => Ok(false),
    }
}

pub(super) async fn link_holder(
    env: &Env<'_>,
    purpose: TokenPurpose,
    token: &str,
) -> Result<Option<User>, sqlx::Error> {
    let user_id = query_get_valid_user_token(purpose, &hash_secret(token), env.db()).await?;
    match user_id {
        Some(user_id) => query_get_user_by_id(user_id, env.db()).await,
        None => Ok(None),
    }
}

pub(super) async fn asks_for_code(
    env: &Env<'_>,
    purpose: TokenPurpose,
    holder: &User,
) -> Result<bool, sqlx::Error> {
    if purpose != TokenPurpose::PasswordReset {
        return Ok(false);
    }
    needs_second_factor(env, holder).await
}
