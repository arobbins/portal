use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;
use crate::web::{CurrentUser, db, globals, session};

use topcoat::{Result, context::Cx, router::error::redirect};
use tower_sessions::Session;

pub async fn current_user(cx: &Cx) -> Result<Option<User>> {
    let user_id: Option<i32> = session(cx)?.get("user_id").await?;

    let Some(user_id) = user_id else {
        return Ok(None);
    };

    Ok(query_get_user_by_id(&user_id, db(cx)).await?)
}

pub async fn require_user(cx: &Cx) -> Result<User> {
    match current_user(cx).await? {
        Some(user) => Ok(user),
        None => Err(redirect("/login").into()),
    }
}

pub fn redirect_signed_in_users(cx: &Cx) -> Result<()> {
    if globals(cx).current_user.is_some() {
        return Err(redirect("/account").into());
    }

    Ok(())
}

pub async fn sign_in(session: &Session, user: &User) -> Result<()> {
    session.cycle_id().await?;
    session.insert("user_id", user.id).await?;
    session
        .insert("current_user", CurrentUser { name: user.name.clone() })
        .await?;

    Ok(())
}
