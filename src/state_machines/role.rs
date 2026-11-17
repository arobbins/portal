use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleState {
    New,
    Subscriber,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleEvent {
    Register,
    Grant,
    Revoke,
}

use RoleEvent as E;
use RoleState as S;

pub const ROLE: Machine<RoleState, RoleEvent> = Machine {
    name: "Role",
    summary: "What someone may do: every account starts as a subscriber; only the server console makes or unmakes admins.",
    states: &[S::New, S::Subscriber, S::Admin],
    terminal: &[],
    events: &[E::Register, E::Grant, E::Revoke],
    stored_in: &["users.role"],
    rules: &[
        ONLY_ADMINS_ADMINISTER,
        CHANGES_ARE_RECORDED,
        REPEATS_CHANGE_NOTHING,
        STAYS_SIGNED_IN,
    ],
    writers: &[
        Writer {
            caller: "src/admin/console.rs:change_role",
            query: "query_set_role_by_email",
            events: &[E::Grant, E::Revoke],
        },
        Writer {
            caller: "src/seed.rs:run",
            query: "query_make_admin",
            events: &[],
        },
    ],
};

pub const ONLY_ADMINS_ADMINISTER: &str =
    "Only an admin may administer users, and only from a browser, never with an API token.";
pub const CHANGES_ARE_RECORDED: &str =
    "Every role change records an account event and an admin log entry.";
pub const REPEATS_CHANGE_NOTHING: &str = "Granting or revoking a role that's already set, or for an address with no account, changes nothing: no event and no log entry.";
pub const STAYS_SIGNED_IN: &str =
    "A role change doesn't sign anyone out: access is checked again on every request.";

pub fn expect(from: RoleState, event: RoleEvent) -> Expect<RoleState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (S::New, E::Register) => Moves(S::Subscriber),
        (S::Subscriber, E::Grant) => Moves(S::Admin),
        (S::Admin, E::Revoke) => Moves(S::Subscriber),
        (S::New, E::Grant | E::Revoke)
        | (S::Subscriber, E::Register | E::Revoke)
        | (S::Admin, E::Register | E::Grant) => Refused,
    }
}

impl Labeled for RoleState {
    fn id(self) -> &'static str {
        match self {
            S::New => "new",
            S::Subscriber => "subscriber",
            S::Admin => "admin",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::New => "No account",
            S::Subscriber => "Subscriber",
            S::Admin => "Admin",
        }
    }
}

impl Labeled for RoleEvent {
    fn id(self) -> &'static str {
        match self {
            E::Register => "register",
            E::Grant => "grant",
            E::Revoke => "revoke",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Register => "Register with this email",
            E::Grant => "webserver admin grant",
            E::Revoke => "webserver admin revoke",
        }
    }
}

#[cfg(test)]
mod tests;
