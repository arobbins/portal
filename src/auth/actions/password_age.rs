use std::convert::Infallible;

use time::OffsetDateTime;

use crate::action::{ActionResult, Caller, authorize};
use crate::env::Env;
use crate::permission::Permission;
use crate::user::event_history::query_last_account_event_at;
use crate::user::events::{AccountEventKind, EVENT_RETENTION};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordAge {
    Changed(OffsetDateTime),
    Set(OffsetDateTime),
    OverAYear,
}

pub async fn password_age(env: &Env<'_>, caller: &Caller) -> ActionResult<PasswordAge, Infallible> {
    authorize!(caller.require(Permission::ManagePassword));
    let kinds = [
        AccountEventKind::PasswordChanged,
        AccountEventKind::PasswordReset,
    ];
    let changed = query_last_account_event_at(caller.user.id, &kinds, env.db()).await?;

    Ok(Ok(age_of(changed, caller.user.verified_at, env.now())))
}

fn age_of(
    changed: Option<OffsetDateTime>,
    set: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> PasswordAge {
    match (changed, set) {
        (Some(changed), _) => PasswordAge::Changed(changed),
        (None, Some(set)) if now - set < EVENT_RETENTION => PasswordAge::Set(set),
        _ => PasswordAge::OverAYear,
    }
}

#[cfg(test)]
mod tests {
    use time::Duration;

    use super::*;

    #[test]
    fn a_recorded_change_wins_then_the_first_set_then_unknown() {
        let now = OffsetDateTime::from_unix_timestamp(1_795_000_000).unwrap();
        let week_ago = now - Duration::days(7);
        let long_ago = now - Duration::days(800);

        assert_eq!(
            age_of(Some(week_ago), Some(long_ago), now),
            PasswordAge::Changed(week_ago)
        );
        assert_eq!(
            age_of(None, Some(week_ago), now),
            PasswordAge::Set(week_ago)
        );
        assert_eq!(age_of(None, Some(long_ago), now), PasswordAge::OverAYear);
        assert_eq!(age_of(None, None, now), PasswordAge::OverAYear);
    }
}
