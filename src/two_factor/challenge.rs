use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tower_sessions::Session;

use crate::devices::lifecycle::SignInMethod;
use crate::env::{Env, is_within};
use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;

pub const PENDING_SECOND_FACTOR: &str = "pending_second_factor";
pub const PENDING_WINDOW: time::Duration = time::Duration::minutes(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PendingSecondFactor {
    user_id: i32,
    session_version: i32,
    started_at: i64,
    method: SignInMethod,
}

pub async fn begin_second_factor(
    session: &Session,
    user: &User,
    method: SignInMethod,
    now: OffsetDateTime,
) -> topcoat::Result<()> {
    let pending = PendingSecondFactor {
        user_id: user.id,
        session_version: user.session_version,
        started_at: now.unix_timestamp(),
        method,
    };
    session.insert(PENDING_SECOND_FACTOR, pending).await?;
    Ok(())
}

pub async fn pending_user(
    env: &Env<'_>,
    session: &Session,
) -> topcoat::Result<Option<(User, SignInMethod)>> {
    let Some(pending) = session
        .get::<PendingSecondFactor>(PENDING_SECOND_FACTOR)
        .await?
    else {
        return Ok(None);
    };
    if !is_within(pending.started_at, env.now(), PENDING_WINDOW) {
        end_second_factor(session).await?;
        return Ok(None);
    }
    let user = query_get_user_by_id(pending.user_id, env.db()).await?;
    Ok(user
        .filter(|user| user.session_version == pending.session_version && user.is_active())
        .map(|user| (user, pending.method)))
}

pub async fn end_second_factor(session: &Session) -> topcoat::Result<()> {
    session
        .remove::<PendingSecondFactor>(PENDING_SECOND_FACTOR)
        .await?;
    Ok(())
}
