use time::{Duration, OffsetDateTime};
use topcoat::Result;
use tower_sessions::Session;

use crate::env::is_within;

const CHECKOUT_PENDING: &str = "checkout_pending";
const PENDING_LASTS: Duration = Duration::hours(1);

pub async fn mark_checkout_pending(session: &Session, now: OffsetDateTime) -> Result<()> {
    session
        .insert(CHECKOUT_PENDING, now.unix_timestamp())
        .await?;
    Ok(())
}

pub async fn checkout_pending(session: &Session, now: OffsetDateTime) -> Result<bool> {
    let marked = session.get::<i64>(CHECKOUT_PENDING).await?;
    Ok(marked.is_some_and(|at| is_within(at, now, PENDING_LASTS)))
}

pub async fn clear_checkout_pending(session: &Session) -> Result<()> {
    session.remove::<i64>(CHECKOUT_PENDING).await?;
    Ok(())
}
