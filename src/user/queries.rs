use sqlx::{PgExecutor, PgPool};
use tower_sessions::cookie::time;
use crate::user::model::User;

pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub password_hash: &'a str,
}

pub async fn query_update_user_verified_time(user_id: i32, db: &PgPool) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as!(
        User,
        "UPDATE users SET verified_at = now() WHERE id = $1 RETURNING *",
        user_id
    )
    .fetch_one(db)
    .await?;

    Ok(user)
}

pub async fn query_delete_email_verification_token(user_id: i32, db: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM email_verification_tokens WHERE user_id = $1", user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn query_get_user_by_email(email: &str, db: &PgPool) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as!(User, "select * from users where email = $1", email)
        .fetch_optional(db)
        .await?;

    Ok(user)
}

pub async fn query_get_user_by_id(user_id: &i32, db: &PgPool) -> Result<Option<User>, sqlx::Error> {

    let user = sqlx::query_as!(User, "select * from users where id = $1", user_id)
        .fetch_optional(db)
        .await?;

    Ok(user)
    
}

pub async fn query_get_user_id_by_token_hash(
    db: &PgPool,
    token_hash: &str,
) -> Result<Option<i32>, sqlx::Error> {
    let record = sqlx::query!(
        "SELECT user_id FROM email_verification_tokens WHERE token_hash = $1 AND expires_at > now()",
        token_hash,
    )
    .fetch_optional(db)
    .await?;

    Ok(record.map(|r| r.user_id))
}

pub async fn query_insert_user(new_user: &NewUser<'_>, db: impl PgExecutor<'_>) -> Result<i32, sqlx::Error> {
    let user_id = sqlx::query_scalar!(
        "INSERT INTO users (name, email, password) VALUES ($1, $2, $3) RETURNING id",
        new_user.name,
        new_user.email,
        new_user.password_hash,
    )
    .fetch_one(db)
    .await?;

    Ok(user_id)
}

pub async fn query_insert_email_verification_token(
    user_id: i32,
    token_hash: &str,
    expires_at: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO email_verification_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
        user_id,
        token_hash,
        expires_at,
    )
    .execute(db)
    .await?;

    Ok(())
}