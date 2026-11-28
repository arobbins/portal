use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use topcoat::Result;
use tower_sessions::Session;

use crate::env::is_within;
use crate::user::model::User;

pub const PASSKEY_PROMPT_GRANT: &str = "passkey_prompt_grant";
pub const PASSKEY_PROMPT_WINDOW: time::Duration = time::Duration::minutes(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PasskeyPromptGrant {
    user_id: i32,
    granted_at: i64,
}

pub async fn grant_passkey_prompt(
    session: &Session,
    user: &User,
    now: OffsetDateTime,
) -> Result<()> {
    session
        .insert(
            PASSKEY_PROMPT_GRANT,
            PasskeyPromptGrant {
                user_id: user.id,
                granted_at: now.unix_timestamp(),
            },
        )
        .await?;

    Ok(())
}

pub async fn passkey_prompt_granted(
    session: &Session,
    user: &User,
    now: OffsetDateTime,
) -> Result<bool> {
    let grant = session
        .get::<PasskeyPromptGrant>(PASSKEY_PROMPT_GRANT)
        .await?;

    Ok(grant.is_some_and(|grant| {
        grant.user_id == user.id && is_within(grant.granted_at, now, PASSKEY_PROMPT_WINDOW)
    }))
}

pub async fn revoke_passkey_prompt(session: &Session) -> Result<()> {
    session
        .remove::<PasskeyPromptGrant>(PASSKEY_PROMPT_GRANT)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_passkey_prompt_grant_lasts_five_minutes() {
        let at = |seconds| time::OffsetDateTime::from_unix_timestamp(seconds).unwrap();
        assert!(is_within(1_000, at(1_000), PASSKEY_PROMPT_WINDOW));
        assert!(is_within(1_000, at(1_300), PASSKEY_PROMPT_WINDOW));
        assert!(!is_within(1_000, at(1_301), PASSKEY_PROMPT_WINDOW));
        assert!(!is_within(1_000, at(999), PASSKEY_PROMPT_WINDOW));
    }
}
