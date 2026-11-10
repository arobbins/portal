use sqlx::PgPool;

use crate::mail::{Email, Mailer};
use crate::user::queries::query_consume_user_token;
use crate::user::tokens::{TokenPurpose, replace_user_token};
use crate::utils::get_token_hash_from_token_string;

pub async fn email_user_link(
    db: &PgPool,
    mailer: &Mailer,
    user_id: i32,
    purpose: TokenPurpose,
    email: impl FnOnce(&str) -> Email,
) -> Result<Result<(), String>, sqlx::Error> {
    let mut tx = db.begin().await?;
    let token = replace_user_token(user_id, purpose, &mut tx).await?;
    tx.commit().await?;

    let sent = mailer.send(&email(&token)).await;
    if sent.is_err() {
        query_consume_user_token(purpose, &get_token_hash_from_token_string(&token), db).await?;
    }

    Ok(sent)
}
