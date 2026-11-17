use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenState {
    New,
    Active,
    Suspended,
    Expired,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenEvent {
    Create,
    Use,
    Revoke,
    ChangePassword,
    PasswordReset,
    AdminDisables,
    AdminEnables,
    Expires,
    CleanupAWeekLater,
    AccountDeleted,
}

use TokenEvent as E;
use TokenState as S;

pub const API_TOKEN: Machine<TokenState, TokenEvent> = Machine {
    name: "API token",
    summary: "Whether one API token gets into the API: from creation, through use, to expiry, revocation or its owner being disabled.",
    states: &[S::New, S::Active, S::Suspended, S::Expired, S::Deleted],
    terminal: &[S::Deleted],
    events: &[
        E::Create,
        E::Use,
        E::Revoke,
        E::ChangePassword,
        E::PasswordReset,
        E::AdminDisables,
        E::AdminEnables,
        E::Expires,
        E::CleanupAWeekLater,
        E::AccountDeleted,
    ],
    stored_in: &["api_tokens (row)", "users.disabled_at"],
    rules: &[
        ONLY_THE_HASH_IS_STORED,
        REJECTIONS_LOOK_THE_SAME,
        OWN_AUDIENCE_ONLY,
        CHANGES_ARE_RECORDED,
        USE_IS_STAMPED,
    ],
    writers: &[
        writer(
            "src/api_tokens/actions/create.rs:store_token",
            "query_insert_api_token",
            &[E::Create],
        ),
        writer(
            "src/api_tokens/actions/manage.rs:revoke_api_token",
            "query_delete_api_token",
            &[E::Revoke],
        ),
        writer(
            "src/api_tokens/lifecycle.rs:revoke_all_for_user",
            "query_delete_user_api_tokens",
            &[E::PasswordReset],
        ),
        writer(
            "src/api_tokens/lifecycle.rs:delete_long_expired",
            "query_delete_expired_api_tokens",
            &[E::CleanupAWeekLater],
        ),
        writer(
            "src/admin/actions/manage.rs:disable_user",
            "query_disable_user",
            &[E::AdminDisables],
        ),
        writer(
            "src/admin/actions/manage.rs:enable_user",
            "query_enable_user",
            &[E::AdminEnables],
        ),
    ],
};

pub const ONLY_THE_HASH_IS_STORED: &str =
    "Only a hash of the token is stored; the token itself never reaches the database.";
pub const REJECTIONS_LOOK_THE_SAME: &str = "Every refused token gets the same 401 invalid_token answer, whatever the reason, so a caller learns nothing about why.";
pub const OWN_AUDIENCE_ONLY: &str =
    "A token works only for its own audience: an API token is refused by the MCP endpoint.";
pub const CHANGES_ARE_RECORDED: &str = "Creating and revoking a token each record one account event (a deleted account keeps no events).";
pub const USE_IS_STAMPED: &str =
    "A successful call stamps when the token was last used; a refused one leaves it alone.";

const NO_TOKEN: &str = "There's no token yet.";
const NO_SESSION: &str = "A disabled account can't sign in to manage tokens.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [TokenEvent],
) -> Writer<TokenEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: TokenState, event: TokenEvent) -> Expect<TokenState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::New, E::Create) => Moves(S::Active),
        (S::New, E::Use) => Refused,
        (S::New, _) => Unreachable(NO_TOKEN),

        (S::Active, E::Use) => Moves(S::Active),
        (S::Active, E::Revoke | E::PasswordReset | E::AccountDeleted) => Moves(S::Deleted),
        (S::Active, E::AdminDisables) => Moves(S::Suspended),
        (S::Active, E::Expires) => Moves(S::Expired),
        (S::Active, E::Create | E::ChangePassword | E::AdminEnables | E::CleanupAWeekLater) => {
            Refused
        }

        (S::Suspended, E::Create | E::Revoke | E::ChangePassword) => Unreachable(NO_SESSION),
        (S::Suspended, E::AdminEnables) => Moves(S::Active),
        (S::Suspended, E::Expires) => Moves(S::Expired),
        (S::Suspended, E::AccountDeleted) => Moves(S::Deleted),
        (S::Suspended, E::Use | E::PasswordReset | E::AdminDisables | E::CleanupAWeekLater) => {
            Refused
        }

        (S::Expired, E::Revoke | E::PasswordReset | E::CleanupAWeekLater | E::AccountDeleted) => {
            Moves(S::Deleted)
        }
        (
            S::Expired,
            E::Create
            | E::Use
            | E::ChangePassword
            | E::AdminDisables
            | E::AdminEnables
            | E::Expires,
        ) => Refused,

        (S::Deleted, _) => Refused,
    }
}

impl Labeled for TokenState {
    fn id(self) -> &'static str {
        match self {
            S::New => "new",
            S::Active => "active",
            S::Suspended => "suspended",
            S::Expired => "expired",
            S::Deleted => "deleted",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::New => "Not created",
            S::Active => "Active",
            S::Suspended => "Owner disabled",
            S::Expired => "Expired",
            S::Deleted => "Deleted",
        }
    }
}

impl Labeled for TokenEvent {
    fn id(self) -> &'static str {
        match self {
            E::Create => "create",
            E::Use => "use",
            E::Revoke => "revoke",
            E::ChangePassword => "change_password",
            E::PasswordReset => "password_reset",
            E::AdminDisables => "admin_disables",
            E::AdminEnables => "admin_enables",
            E::Expires => "expires",
            E::CleanupAWeekLater => "cleanup_a_week_later",
            E::AccountDeleted => "account_deleted",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Create => "Owner creates a token",
            E::Use => "Calls the API with it",
            E::Revoke => "Owner revokes it",
            E::ChangePassword => "Owner changes the password",
            E::PasswordReset => "Password reset by email link",
            E::AdminDisables => "Admin disables the owner",
            E::AdminEnables => "Admin enables the owner",
            E::Expires => "Its lifetime runs out",
            E::CleanupAWeekLater => "Hourly cleanup, a week after expiry",
            E::AccountDeleted => "Owner's account deleted",
        }
    }
}

#[cfg(test)]
mod tests;
