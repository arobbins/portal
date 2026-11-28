use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    New,
    Live,
    Expired,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkEvent {
    Issue,
    Open,
    Use,
    Expires,
    Cleanup,
    EmailChanged,
    AdminDisables,
}

use LinkEvent as E;
use LinkState as S;

pub const EMAILED_LINK: Machine<LinkState, LinkEvent> = Machine {
    name: "Emailed link",
    summary: "One emailed link (a password reset link here; every purpose shares the same table and code): sent, opened, used once, replaced or expired.",
    states: &[S::New, S::Live, S::Expired, S::Dead],
    terminal: &[S::Dead],
    events: &[
        E::Issue,
        E::Open,
        E::Use,
        E::Expires,
        E::Cleanup,
        E::EmailChanged,
        E::AdminDisables,
    ],
    stored_in: &["user_tokens (row)"],
    rules: &[
        ONLY_THE_HASH_IS_STORED,
        OPENING_KEEPS_IT,
        DEAD_ENDS_LOOK_THE_SAME,
        ONE_LIVE_LINK,
    ],
    writers: &[
        writer(REPLACE, "query_insert_user_token", &[E::Issue]),
        writer(REPLACE, "query_delete_user_tokens", &[E::Issue]),
        writer(CONSUME, "query_consume_user_token", &[E::Use]),
        writer(CONSUME, "query_delete_user_tokens", &[E::Use]),
        writer(SWITCH, "query_delete_user_tokens", &[E::EmailChanged]),
        writer(DISABLE, "query_delete_all_user_tokens", &[E::AdminDisables]),
        writer(CLEANUP, "query_delete_expired_user_tokens", &[E::Cleanup]),
        writer(CONFIRM, "query_consume_user_token_payload", &[]),
        writer(RESET, "query_delete_user_tokens", &[]),
        writer(CHANGE, "query_delete_user_tokens", &[]),
    ],
};

const REPLACE: &str = "src/auth/links.rs:replace_user_token";
const CONSUME: &str = "src/auth/actions/link_password.rs:consume_link";
const SWITCH: &str = "src/auth/actions/confirm_email.rs:switch_email";
const CONFIRM: &str = "src/auth/actions/confirm_email.rs:confirm_email_change";
const RESET: &str = "src/auth/actions/link_password.rs:revoke_access";
const CHANGE: &str = "src/auth/actions/change_password.rs:change_password";
const DISABLE: &str = "src/auth/links.rs:delete_every_link";
const CLEANUP: &str = "src/auth/links.rs:delete_stale_sign_ups";

pub const ONLY_THE_HASH_IS_STORED: &str =
    "Only a hash of the link's token is stored; the token itself never reaches the database.";
pub const OPENING_KEEPS_IT: &str =
    "Opening a live link only shows the form; only submitting it uses it up.";
pub const DEAD_ENDS_LOOK_THE_SAME: &str = "Every expired, used or replaced link shows the same 'invalid or already used' page, so it reveals nothing.";
pub const ONE_LIVE_LINK: &str = "An account has at most one live link of each kind.";

const NO_LINK: &str = "No link has been sent yet.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [LinkEvent],
) -> Writer<LinkEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: LinkState, event: LinkEvent) -> Expect<LinkState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::New, E::Issue) => Moves(S::Live),
        (S::New, E::Open | E::Use) => Refused,
        (S::New, _) => Unreachable(NO_LINK),
        (S::Live, E::Expires) => Moves(S::Expired),
        (S::Live, E::Issue | E::Use | E::EmailChanged | E::AdminDisables) => Moves(S::Dead),
        (S::Expired, E::Issue | E::Cleanup | E::EmailChanged | E::AdminDisables) => Moves(S::Dead),
        _ => Refused,
    }
}

impl Labeled for LinkState {
    fn id(self) -> &'static str {
        match self {
            S::New => "new",
            S::Live => "live",
            S::Expired => "expired",
            S::Dead => "dead",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::New => "Not sent",
            S::Live => "Live",
            S::Expired => "Expired",
            S::Dead => "Used or replaced",
        }
    }
}

impl Labeled for LinkEvent {
    fn id(self) -> &'static str {
        match self {
            E::Issue => "issue",
            E::Open => "open",
            E::Use => "use",
            E::Expires => "expires",
            E::Cleanup => "cleanup",
            E::EmailChanged => "email_changed",
            E::AdminDisables => "admin_disables",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Issue => "A new link is emailed",
            E::Open => "Opens the link",
            E::Use => "Submits the form",
            E::Expires => "Its lifetime runs out",
            E::Cleanup => "Hourly cleanup",
            E::EmailChanged => "An email change is confirmed",
            E::AdminDisables => "Admin disables the account",
        }
    }
}

#[cfg(test)]
mod tests;
