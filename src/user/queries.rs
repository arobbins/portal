use sqlx::PgExecutor;

use crate::user::model::{Role, User};

pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
}

pub async fn query_insert_user(
    new_user: &NewUser<'_>,
    db: impl PgExecutor<'_>,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO users (name, email) VALUES ($1, $2) ON CONFLICT (email) DO NOTHING RETURNING id",
        new_user.name,
        new_user.email,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_get_user_by_id(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id FROM users WHERE id = $1",
        user_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_get_user_by_email(
    email: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id FROM users WHERE email = $1",
        email,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_get_user_by_webauthn_id(
    webauthn_id: uuid::Uuid,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id FROM users WHERE webauthn_id = $1",
        webauthn_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_update_user_name(
    user_id: i32,
    name: &str,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET name = $1, updated_at = now() WHERE id = $2",
        name,
        user_id
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_update_user_email(
    user_id: i32,
    email: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    let updated = sqlx::query_as!(
        User,
        "UPDATE users SET email = $1, session_version = session_version + 1, updated_at = now() WHERE id = $2 AND verified_at IS NOT NULL AND NOT EXISTS (SELECT 1 FROM users WHERE email = $1) RETURNING id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id",
        email,
        user_id,
    )
    .fetch_optional(db)
    .await;

    match updated {
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(None),
        other => other,
    }
}

pub async fn query_set_password_and_verify_user(
    user_id: i32,
    password_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        "UPDATE users SET password = $1, verified_at = now(), session_version = session_version + 1, updated_at = now() WHERE id = $2 RETURNING id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id",
        password_hash,
        user_id,
    )
    .fetch_one(db)
    .await
}

pub async fn query_update_password(
    user_id: i32,
    password_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "UPDATE users SET password = $1, session_version = session_version + 1, updated_at = now() WHERE id = $2 AND verified_at IS NOT NULL RETURNING id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id",
        password_hash,
        user_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_delete_abandoned_signups(
    cutoff: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM users WHERE password IS NULL AND created_at < $1 AND NOT EXISTS (SELECT 1 FROM user_tokens WHERE user_tokens.user_id = users.id AND user_tokens.expires_at > now())",
        cutoff,
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}
