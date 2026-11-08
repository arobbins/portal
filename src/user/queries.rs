use crate::user::model::User;
use crate::user::tokens::TokenPurpose;
use sqlx::{PgExecutor, PgPool};

pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub password_hash: Option<&'a str>,
}

pub async fn query_update_user_verified_time(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as!(
        User,
        "UPDATE users SET verified_at = now() WHERE id = $1 RETURNING *",
        user_id
    )
    .fetch_one(db)
    .await?;

    Ok(user)
}

pub async fn query_update_user_name(
    user_id: i32,
    name: &str,
    db: &PgPool,
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

pub async fn query_get_user_by_email(
    email: &str,
    db: &PgPool,
) -> Result<Option<User>, sqlx::Error> {
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

pub async fn query_consume_user_token(
    purpose: TokenPurpose,
    token_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<i32>, sqlx::Error> {
    let user_id = sqlx::query_scalar!(
        "DELETE FROM user_tokens WHERE purpose = $1 AND token_hash = $2 AND expires_at > now() RETURNING user_id",
        purpose.as_str(),
        token_hash,
    )
    .fetch_optional(db)
    .await?;

    Ok(user_id)
}

pub async fn query_insert_user(
    new_user: &NewUser<'_>,
    db: impl PgExecutor<'_>,
) -> Result<i32, sqlx::Error> {
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

pub async fn query_find_valid_user_token(
    purpose: TokenPurpose,
    token_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<i32>, sqlx::Error> {
    let user_id = sqlx::query_scalar!(
        "SELECT user_id FROM user_tokens WHERE purpose = $1 AND token_hash = $2 AND expires_at > now()",
        purpose.as_str(),
        token_hash,
    )
    .fetch_optional(db)
    .await?;

    Ok(user_id)
}

pub async fn query_set_password_and_verify_user(
    user_id: i32,
    password_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as!(
        User,
        "UPDATE users SET password = $1, verified_at = now(), updated_at = now() WHERE id = $2 RETURNING *",
        password_hash,
        user_id,
    )
    .fetch_one(db)
    .await?;

    Ok(user)
}

pub async fn query_insert_user_token(
    user_id: i32,
    purpose: TokenPurpose,
    token_hash: &str,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO user_tokens (user_id, purpose, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        user_id,
        purpose.as_str(),
        token_hash,
        purpose.expires_at(),
    )
    .execute(db)
    .await?;

    Ok(())
}
