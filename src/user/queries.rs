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
        "SELECT id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id, public_id, disabled_at, admin_granted_at, time_zone FROM users WHERE id = $1",
        user_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_get_user_created_at(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<time::OffsetDateTime, sqlx::Error> {
    sqlx::query_scalar!("SELECT created_at FROM users WHERE id = $1", user_id)
        .fetch_one(db)
        .await
}

pub async fn query_get_user_by_email(
    email: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id, public_id, disabled_at, admin_granted_at, time_zone FROM users WHERE email = $1",
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
        "SELECT id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id, public_id, disabled_at, admin_granted_at, time_zone FROM users WHERE webauthn_id = $1",
        webauthn_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_update_user_time_zone(
    user_id: i32,
    time_zone: &str,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET time_zone = $1, updated_at = now() WHERE id = $2",
        time_zone,
        user_id
    )
    .execute(db)
    .await?;

    Ok(())
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
        "UPDATE users SET email = $1, session_version = session_version + 1, updated_at = now() WHERE id = $2 AND verified_at IS NOT NULL AND NOT EXISTS (SELECT 1 FROM users WHERE email = $1) RETURNING id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id, public_id, disabled_at, admin_granted_at, time_zone",
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
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "UPDATE users SET password = $1, verified_at = now(), session_version = session_version + 1, updated_at = now() WHERE id = $2 AND password IS NULL RETURNING id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id, public_id, disabled_at, admin_granted_at, time_zone",
        password_hash,
        user_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_update_password(
    user_id: i32,
    password_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "UPDATE users SET password = $1, session_version = session_version + 1, updated_at = now() WHERE id = $2 AND verified_at IS NOT NULL RETURNING id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id, public_id, disabled_at, admin_granted_at, time_zone",
        password_hash,
        user_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_delete_user(user_id: i32, db: impl PgExecutor<'_>) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(db)
        .await?;

    Ok(result.rows_affected() == 1)
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

pub async fn query_bump_session_version(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        "UPDATE users SET session_version = session_version + 1, updated_at = now() WHERE id = $1 RETURNING id, name, email, password, role AS \"role: Role\", verified_at, session_version, webauthn_id, public_id, disabled_at, admin_granted_at, time_zone",
        user_id,
    )
    .fetch_one(db)
    .await
}

pub async fn query_get_user_by_public_id(
    public_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"SELECT id, name, email, password, role AS "role: Role", verified_at, session_version, webauthn_id, public_id, disabled_at, admin_granted_at, time_zone FROM users WHERE public_id = $1"#,
        public_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_lock_user_by_public_id(
    public_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"SELECT id, name, email, password, role AS "role: Role", verified_at, session_version, webauthn_id, public_id, disabled_at, admin_granted_at, time_zone FROM users WHERE public_id = $1 FOR UPDATE"#,
        public_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_disable_user(
    user_id: i32,
    disabled_at: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET disabled_at = $1, session_version = session_version + 1, updated_at = now() WHERE id = $2",
        disabled_at,
        user_id,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_enable_user(user_id: i32, db: impl PgExecutor<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET disabled_at = NULL, updated_at = now() WHERE id = $1",
        user_id,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_set_role_by_email(
    email: &str,
    role: Role,
    db: impl PgExecutor<'_>,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE users SET role = $1, admin_granted_at = CASE WHEN $1 = 'admin' THEN now() END, updated_at = now() WHERE email = $2 AND role <> $1 RETURNING id",
        role as Role,
        email,
    )
    .fetch_optional(db)
    .await
}
