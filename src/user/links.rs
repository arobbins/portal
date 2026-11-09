use sqlx::{Postgres, Transaction};
use topcoat::{Result, context::Cx};

use crate::mail::Email;
use crate::user::tokens::{TokenPurpose, replace_user_token};
use crate::web::mailer;

pub async fn email_user_link(
    cx: &Cx,
    mut tx: Transaction<'_, Postgres>,
    user_id: i32,
    purpose: TokenPurpose,
    email: impl FnOnce(&str) -> Result<Email>,
) -> Result<std::result::Result<(), String>> {
    let token = replace_user_token(user_id, purpose, &mut tx).await?;
    let sent = mailer(cx).send(&email(&token)?).await;

    if sent.is_ok() {
        tx.commit().await?;
    }

    Ok(sent)
}
