use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Method,
        error::{not_found, see_other},
        href, request,
    },
};
use tower_sessions::Session;

use super::admin_confirm::admin_confirm_page;
use crate::action::{Caller, browser_caller};
use crate::permission::Permission;
use crate::user::model::User;
use crate::web::{app, session};

const ADMIN_UNLOCK: &str = "admin_unlock";
const ADMIN_RETURN_TO: &str = "admin_return_to";
const ADMIN_PATH_PREFIX: &str = "/admin/";
pub const ADMIN_IDLE_LIMIT: Duration = Duration::minutes(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct AdminUnlock {
    user_id: i32,
    session_version: i32,
    #[serde(default)]
    admin_since: Option<i64>,
    last_used_at: i64,
}

pub fn admin_caller(cx: &Cx) -> Result<Caller> {
    match browser_caller(cx) {
        Some(caller) if caller.require(Permission::AdministerUsers).is_ok() => Ok(caller),
        _ => Err(not_found().into()),
    }
}

pub async fn require_admin(cx: &Cx) -> Result<Caller> {
    let caller = admin_caller(cx)?;
    let session = session(cx)?;
    let now = app(cx).clock.now();
    if !is_unlocked(&session, &caller.user, now).await? {
        if request::method(cx) == Method::GET {
            let path = request::uri(cx)
                .path_and_query()
                .map_or("", |path| path.as_str());
            session.insert(ADMIN_RETURN_TO, path.to_string()).await?;
        }
        return Err(see_other(href!(admin_confirm_page).resolve(cx)).into());
    }
    unlock_admin(&session, &caller.user, now).await?;

    Ok(caller)
}

pub async fn unlock_admin(session: &Session, user: &User, now: OffsetDateTime) -> Result<()> {
    let unlock = AdminUnlock {
        user_id: user.id,
        session_version: user.session_version,
        admin_since: admin_since(user),
        last_used_at: now.unix_timestamp(),
    };
    session.insert(ADMIN_UNLOCK, unlock).await?;
    Ok(())
}

pub async fn take_return_to(session: &Session) -> Result<Option<String>> {
    let path = session.remove::<String>(ADMIN_RETURN_TO).await?;
    Ok(path.filter(|path| is_admin_path(path)))
}

fn is_admin_path(path: &str) -> bool {
    path.starts_with(ADMIN_PATH_PREFIX)
}

async fn is_unlocked(session: &Session, user: &User, now: OffsetDateTime) -> Result<bool> {
    let unlock = session.get::<AdminUnlock>(ADMIN_UNLOCK).await?;

    Ok(unlock.is_some_and(|unlock| still_unlocked(unlock, user, now)))
}

fn still_unlocked(unlock: AdminUnlock, user: &User, now: OffsetDateTime) -> bool {
    let idle = now.unix_timestamp() - unlock.last_used_at;
    unlock.user_id == user.id
        && unlock.session_version == user.session_version
        && unlock.admin_since.is_some()
        && unlock.admin_since == admin_since(user)
        && (0..=ADMIN_IDLE_LIMIT.whole_seconds()).contains(&idle)
}

fn admin_since(user: &User) -> Option<i64> {
    user.admin_granted_at
        .map(|granted_at| (granted_at.unix_timestamp_nanos() / 1_000) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_admin_area_locks_after_fifteen_idle_minutes_or_a_new_sign_in() {
        let user = admin_granted_at(OffsetDateTime::UNIX_EPOCH);
        let unlock = AdminUnlock {
            user_id: user.id,
            session_version: user.session_version,
            admin_since: admin_since(&user),
            last_used_at: 1_000,
        };

        assert!(still_unlocked(unlock, &user, at(1_000 + 15 * 60)));
        assert!(!still_unlocked(unlock, &user, at(1_000 + 15 * 60 + 1)));
        let signed_in_again = AdminUnlock {
            session_version: user.session_version - 1,
            ..unlock
        };
        assert!(!still_unlocked(signed_in_again, &user, at(1_001)));
    }

    #[test]
    fn granting_the_admin_role_again_locks_the_admin_area() {
        let user = admin_granted_at(OffsetDateTime::UNIX_EPOCH);
        let unlock = AdminUnlock {
            user_id: user.id,
            session_version: user.session_version,
            admin_since: admin_since(&user),
            last_used_at: 1_000,
        };
        let granted_again =
            admin_granted_at(OffsetDateTime::UNIX_EPOCH + Duration::microseconds(1));
        let never_granted = User::default_for_tests();

        assert!(still_unlocked(unlock, &user, at(1_001)));
        assert!(!still_unlocked(unlock, &granted_again, at(1_001)));
        assert!(!still_unlocked(
            AdminUnlock {
                admin_since: None,
                ..unlock
            },
            &never_granted,
            at(1_001)
        ));
    }

    fn admin_granted_at(granted_at: OffsetDateTime) -> User {
        User {
            role: crate::user::model::Role::Admin,
            admin_granted_at: Some(granted_at),
            ..User::default_for_tests()
        }
    }

    fn at(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(seconds).unwrap()
    }

    #[test]
    fn only_admin_pages_are_returned_to_after_confirming() {
        assert!(is_admin_path("/admin/users/usr_00000000000000000000"));
        assert!(is_admin_path("/admin/users?q=ada"));
        assert!(!is_admin_path("/account"));
        assert!(!is_admin_path("https://evil.test/admin/"));
        assert!(!is_admin_path("//evil.test/admin/"));
    }
}
