use std::sync::Arc;

use time::{Duration, OffsetDateTime};
use topcoat::{Result, context::Cx};
use tower_sessions::Session;

use crate::action::Caller;
use crate::billing::actions::{StartCheckout, start_checkout};
use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;
use crate::web::env;

const CHECKOUT_PENDING: &str = "checkout_pending";
const PENDING_LASTS: Duration = Duration::hours(1);

pub async fn checkout_for_price(cx: &Cx, user: &User, price: String) -> Result<Option<String>> {
    let env = env(cx);
    let Some(user) = query_get_user_by_id(user.id, env.db()).await? else {
        return Ok(None);
    };
    let caller = Caller::browser(Arc::new(user));
    match start_checkout(&env, &caller, &StartCheckout { price }).await? {
        Ok(link) => Ok(Some(link.checkout_url)),
        Err(error) => {
            tracing::info!("no checkout after sign-in: {:?}", error.for_browser()?);
            Ok(None)
        }
    }
}

pub async fn mark_checkout_pending(session: &Session, now: OffsetDateTime) -> Result<()> {
    session
        .insert(CHECKOUT_PENDING, now.unix_timestamp())
        .await?;
    Ok(())
}

pub async fn checkout_pending(session: &Session, now: OffsetDateTime) -> Result<bool> {
    let marked = session.get::<i64>(CHECKOUT_PENDING).await?;
    Ok(marked
        .and_then(|at| OffsetDateTime::from_unix_timestamp(at).ok())
        .is_some_and(|at| now - at < PENDING_LASTS))
}

pub async fn clear_checkout_pending(session: &Session) -> Result<()> {
    session.remove::<i64>(CHECKOUT_PENDING).await?;
    Ok(())
}
