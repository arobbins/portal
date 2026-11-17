use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeState {
    Unchanged,
    Pending,
    Changed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeEvent {
    Request,
    RequestTakenAddress,
    OpenLink,
    Confirm,
    AddressTakenFirst,
    LinkExpires,
    ChangePassword,
    PasswordReset,
    AdminDisables,
}

use ChangeEvent as E;
use ChangeState as S;

pub const EMAIL_CHANGE: Machine<ChangeState, ChangeEvent> = Machine {
    name: "Email change",
    summary: "Moving an account to a new email address: asked for with the password, confirmed from a link sent to the new address.",
    states: &[S::Unchanged, S::Pending, S::Changed],
    terminal: &[],
    events: &[
        E::Request,
        E::RequestTakenAddress,
        E::OpenLink,
        E::Confirm,
        E::AddressTakenFirst,
        E::LinkExpires,
        E::ChangePassword,
        E::PasswordReset,
        E::AdminDisables,
    ],
    stored_in: &["users.email", "user_tokens (row)"],
    rules: &[
        OPENING_KEEPS_THE_LINK,
        NEWEST_LINK_ONLY,
        SAME_ANSWER_EITHER_WAY,
        CONFIRMING_TELLS_THE_OLD_ADDRESS,
        CONFIRMING_SIGNS_OUT_EVERYWHERE,
    ],
    writers: &[
        writer(SWITCH, "query_update_user_email", &[E::Confirm]),
        writer(SWITCH, "query_delete_user_tokens", &[E::Confirm]),
        writer(CONFIRM, "query_consume_user_token_payload", &[E::Confirm]),
        writer(REPLACE, "query_insert_user_token", &[E::Request]),
        writer(REPLACE, "query_delete_user_tokens", &[E::Request]),
        writer(CHANGE, "query_delete_user_tokens", &[E::ChangePassword]),
        writer(RESET, "query_delete_user_tokens", &[E::PasswordReset]),
        writer(RESET_LINK, "query_consume_user_token", &[E::PasswordReset]),
        writer(RESET_LINK, "query_delete_user_tokens", &[E::PasswordReset]),
        writer(DISABLE, "query_delete_all_user_tokens", &[E::AdminDisables]),
        writer(CLEANUP, "query_delete_expired_user_tokens", &[]),
    ],
};

const SWITCH: &str = "src/auth/actions/confirm_email.rs:switch_email";
const CONFIRM: &str = "src/auth/actions/confirm_email.rs:confirm_email_change";
const REPLACE: &str = "src/auth/links.rs:replace_user_token";
const CHANGE: &str = "src/auth/actions/change_password.rs:change_password";
const RESET: &str = "src/auth/actions/link_password.rs:revoke_access";
const RESET_LINK: &str = "src/auth/actions/link_password.rs:consume_link";
const DISABLE: &str = "src/admin/actions/manage.rs:disable_user";
const CLEANUP: &str = "src/auth/links.rs:delete_stale_sign_ups";

pub const OPENING_KEEPS_THE_LINK: &str =
    "Opening the link only shows the confirm page; only the button press uses it up.";
pub const NEWEST_LINK_ONLY: &str = "Asking again replaces the link: only the newest one works.";
pub const SAME_ANSWER_EITHER_WAY: &str =
    "The request gets the same answer whether or not the new address already has an account.";
pub const CONFIRMING_TELLS_THE_OLD_ADDRESS: &str = "Confirming sends a notice to the old address.";
pub const CONFIRMING_SIGNS_OUT_EVERYWHERE: &str = "Confirming signs out every device.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [ChangeEvent],
) -> Writer<ChangeEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: ChangeState, event: ChangeEvent) -> Expect<ChangeState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (_, E::Request) => Moves(S::Pending),
        (S::Pending, E::Confirm) => Moves(S::Changed),
        (S::Pending, E::LinkExpires | E::ChangePassword | E::PasswordReset | E::AdminDisables) => {
            Moves(S::Unchanged)
        }
        _ => Refused,
    }
}

impl Labeled for ChangeState {
    fn id(self) -> &'static str {
        match self {
            S::Unchanged => "unchanged",
            S::Pending => "pending",
            S::Changed => "changed",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Unchanged => "No change asked for",
            S::Pending => "Waiting for confirmation",
            S::Changed => "Changed",
        }
    }
}

impl Labeled for ChangeEvent {
    fn id(self) -> &'static str {
        match self {
            E::Request => "request",
            E::RequestTakenAddress => "request_taken_address",
            E::OpenLink => "open_link",
            E::Confirm => "confirm",
            E::AddressTakenFirst => "address_taken_first",
            E::LinkExpires => "link_expires",
            E::ChangePassword => "change_password",
            E::PasswordReset => "password_reset",
            E::AdminDisables => "admin_disables",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Request => "Asks to move to a free address",
            E::RequestTakenAddress => "Asks to move to an address in use",
            E::OpenLink => "Opens the link",
            E::Confirm => "Confirms from the link",
            E::AddressTakenFirst => "Someone takes the address, then they confirm",
            E::LinkExpires => "The link runs out",
            E::ChangePassword => "Changes the password",
            E::PasswordReset => "Password reset by email link",
            E::AdminDisables => "Admin disables the account",
        }
    }
}

#[cfg(test)]
mod tests;
