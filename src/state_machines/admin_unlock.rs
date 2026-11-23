use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockState {
    Locked,
    Unlocked,
    NoAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockEvent {
    Confirm,
    ConfirmWrongPassword,
    OpenSoon,
    OpenLate,
    ChangePassword,
    SignInAgain,
    RoleRevoked,
    RoleRevokedAndGranted,
}

use UnlockEvent as E;
use UnlockState as S;

pub const ADMIN_UNLOCK: Machine<UnlockState, UnlockEvent> = Machine {
    name: "Admin unlock",
    summary: "Whether a signed-in admin can use the admin area right now: confirmed recently with a password or passkey, or locked.",
    states: &[S::Locked, S::Unlocked, S::NoAccess],
    terminal: &[S::NoAccess],
    events: &[
        E::Confirm,
        E::ConfirmWrongPassword,
        E::OpenSoon,
        E::OpenLate,
        E::ChangePassword,
        E::SignInAgain,
        E::RoleRevoked,
        E::RoleRevokedAndGranted,
    ],
    stored_in: &["session:ADMIN_UNLOCK"],
    rules: &[RETURNS_TO_PAGE, TIMER_SLIDES, NON_ADMINS_SEE_NOTHING],
    writers: &[
        Writer {
            caller: "src/admin/pages/admin_confirm.rs:admin_confirm_submit",
            query: "unlock_admin",
            events: &[E::Confirm],
        },
        Writer {
            caller: "src/admin/pages/admin_unlock_routes.rs:unlock_admin_js",
            query: "unlock_admin",
            events: &[E::Confirm],
        },
        Writer {
            caller: "src/admin/pages/admin_access.rs:require_admin",
            query: "unlock_admin",
            events: &[E::OpenSoon],
        },
    ],
};

pub const RETURNS_TO_PAGE: &str = "Confirming goes back to the admin page that asked for it.";
pub const TIMER_SLIDES: &str =
    "Every admin page opened restarts the 15-minute timer, so steady use never locks.";
pub const NON_ADMINS_SEE_NOTHING: &str =
    "Without the admin role, both the admin pages and the confirm page are a 404.";

pub const SOON_MINUTES: i64 = 14;
pub const LATE_MINUTES: i64 = 16;

const NOT_AN_ADMIN: &str = "Not an admin any more: every admin page is a 404.";

pub fn expect(from: UnlockState, event: UnlockEvent) -> Expect<UnlockState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::NoAccess, _) => Unreachable(NOT_AN_ADMIN),
        (_, E::RoleRevoked) => Moves(S::NoAccess),

        (S::Locked, E::Confirm) => Moves(S::Unlocked),
        (
            S::Locked,
            E::ConfirmWrongPassword
            | E::OpenSoon
            | E::OpenLate
            | E::ChangePassword
            | E::SignInAgain
            | E::RoleRevokedAndGranted,
        ) => Refused,

        (S::Unlocked, E::Confirm | E::OpenSoon) => Moves(S::Unlocked),
        (S::Unlocked, E::ConfirmWrongPassword) => Refused,
        (
            S::Unlocked,
            E::OpenLate | E::ChangePassword | E::SignInAgain | E::RoleRevokedAndGranted,
        ) => Moves(S::Locked),
    }
}

impl Labeled for UnlockState {
    fn id(self) -> &'static str {
        match self {
            S::Locked => "locked",
            S::Unlocked => "unlocked",
            S::NoAccess => "no_access",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Locked => "Locked",
            S::Unlocked => "Unlocked",
            S::NoAccess => "No access",
        }
    }
}

impl Labeled for UnlockEvent {
    fn id(self) -> &'static str {
        match self {
            E::Confirm => "confirm",
            E::ConfirmWrongPassword => "confirm_wrong_password",
            E::OpenSoon => "open_soon",
            E::OpenLate => "open_late",
            E::ChangePassword => "change_password",
            E::SignInAgain => "sign_in_again",
            E::RoleRevoked => "role_revoked",
            E::RoleRevokedAndGranted => "role_revoked_and_granted",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Confirm => "Confirms with the password",
            E::ConfirmWrongPassword => "Confirms with a wrong password",
            E::OpenSoon => "Opens an admin page 14 minutes later",
            E::OpenLate => "Opens an admin page 16 minutes later",
            E::ChangePassword => "Changes own password",
            E::SignInAgain => "Signs out and back in",
            E::RoleRevoked => "Admin role revoked",
            E::RoleRevokedAndGranted => "Admin role revoked, then granted again",
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits {
    use super::*;
    use crate::admin::pages::admin_access::ADMIN_IDLE_LIMIT;

    #[test]
    fn the_waits_in_the_event_labels_match_the_limit() {
        let limit = ADMIN_IDLE_LIMIT.whole_minutes();
        assert!(SOON_MINUTES < limit && limit < LATE_MINUTES);
        assert!(
            E::OpenSoon
                .label()
                .contains(&format!("{SOON_MINUTES} minutes"))
        );
        assert!(
            E::OpenLate
                .label()
                .contains(&format!("{LATE_MINUTES} minutes"))
        );
    }
}
