use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    New,
    AwaitingPassword,
    Active,
    DisabledWithoutPassword,
    DisabledWithPassword,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountEvent {
    Register,
    ConfirmEmail,
    ResetPassword,
    ChangePassword,
    Disable,
    Enable,
    DeleteSelf,
    DeleteByAdmin,
    Cleanup,
}

use AccountEvent as E;
use AccountState as S;

pub const ACCOUNT: Machine<AccountState, AccountEvent> = Machine {
    name: "Account",
    summary: "Whether someone can sign in: from sign-up, through setting a password, to being disabled or deleted.",
    states: &[
        S::New,
        S::AwaitingPassword,
        S::Active,
        S::DisabledWithoutPassword,
        S::DisabledWithPassword,
        S::Deleted,
    ],
    terminal: &[S::Deleted],
    events: &[
        E::Register,
        E::ConfirmEmail,
        E::ResetPassword,
        E::ChangePassword,
        E::Disable,
        E::Enable,
        E::DeleteSelf,
        E::DeleteByAdmin,
        E::Cleanup,
    ],
    stored_in: &[
        "users.password",
        "users.verified_at",
        "users.disabled_at",
        "users (row)",
    ],
    rules: &[
        DISABLED_HAS_NO_ACCESS,
        AWAITING_HAS_NO_DEVICES,
        DELETED_LEAVES_NOTHING,
        CHANGES_ARE_RECORDED,
        REFUSED_CHANGES_NOTHING,
        SIGN_OUT_EVERYWHERE,
    ],
    writers: &[
        writer(REGISTER, "query_insert_user", &[E::Register]),
        writer(
            LINK,
            "query_set_password_and_verify_user",
            &[E::ConfirmEmail],
        ),
        writer(RESET, "query_update_password", &[E::ResetPassword]),
        writer(CHANGE, "query_update_password", &[E::ChangePassword]),
        writer(ADMIN_DISABLE, "query_disable_user", &[E::Disable]),
        writer(ADMIN_ENABLE, "query_enable_user", &[E::Enable]),
        writer(SELF_DELETE, "query_delete_user", &[E::DeleteSelf]),
        writer(ADMIN_DELETE, "query_delete_user", &[E::DeleteByAdmin]),
        writer(CLEANUP, "query_delete_abandoned_signups", &[E::Cleanup]),
        writer(SEED, "query_insert_user", &[]),
        writer(SEED, "query_set_password_and_verify_user", &[]),
    ],
};

const REGISTER: &str = "src/auth/actions/register.rs:start_new_account";
const LINK: &str = "src/auth/actions/link_password.rs:apply_password";
const RESET: &str = "src/auth/actions/link_password.rs:reset_password";
const CHANGE: &str = "src/auth/actions/change_password.rs:change_password";
const ADMIN_DISABLE: &str = "src/admin/actions/manage.rs:disable_user";
const ADMIN_ENABLE: &str = "src/admin/actions/manage.rs:enable_user";
const ADMIN_DELETE: &str = "src/admin/actions/manage.rs:delete_user";
const SELF_DELETE: &str = "src/auth/actions/delete_account.rs:delete_account";
const CLEANUP: &str = "src/auth/links.rs:delete_stale_sign_ups";
const SEED: &str = "src/seed.rs:run";

pub const DISABLED_HAS_NO_ACCESS: &str =
    "A disabled account has no signed-in devices and no live emailed links.";
pub const AWAITING_HAS_NO_DEVICES: &str =
    "An account awaiting a password has no signed-in devices.";
pub const DELETED_LEAVES_NOTHING: &str =
    "A deleted account leaves nothing behind: no devices, links, passkeys, API tokens or events.";
pub const CHANGES_ARE_RECORDED: &str =
    "Every accepted change records an account event, unless it deletes the account.";
pub const REFUSED_CHANGES_NOTHING: &str =
    "A refused event changes nothing: no new event and the same session version.";
pub const SIGN_OUT_EVERYWHERE: &str = "Setting, resetting or changing a password and disabling sign out every device; nothing else does.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [AccountEvent],
) -> Writer<AccountEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn signs_out_everywhere(event: AccountEvent) -> bool {
    matches!(
        event,
        E::ConfirmEmail | E::ResetPassword | E::ChangePassword | E::Disable
    )
}

const NO_ACCOUNT: &str = "There's no account yet.";
const GONE: &str = "The account is gone.";
const NO_SESSION_YET: &str = "Can't sign in before setting a password.";
const NO_SESSION_DISABLED: &str = "A disabled account has no session.";
const NO_LINK: &str = "Confirmation links exist only while awaiting a password.";

pub fn expect(from: AccountState, event: AccountEvent) -> Expect<AccountState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::New, E::Register) => Moves(S::AwaitingPassword),
        (S::New, _) => Unreachable(NO_ACCOUNT),
        (S::Deleted, _) => Unreachable(GONE),

        (S::AwaitingPassword, E::ConfirmEmail) => Moves(S::Active),
        (S::AwaitingPassword, E::Disable) => Moves(S::DisabledWithoutPassword),
        (S::AwaitingPassword, E::DeleteByAdmin | E::Cleanup) => Moves(S::Deleted),
        (S::AwaitingPassword, E::DeleteSelf | E::ChangePassword) => Unreachable(NO_SESSION_YET),
        (S::AwaitingPassword, E::Register | E::ResetPassword | E::Enable) => Refused,

        (S::Active, E::ResetPassword | E::ChangePassword) => Moves(S::Active),
        (S::Active, E::Disable) => Moves(S::DisabledWithPassword),
        (S::Active, E::DeleteSelf | E::DeleteByAdmin) => Moves(S::Deleted),
        (S::Active, E::ConfirmEmail) => Unreachable(NO_LINK),
        (S::Active, E::Register | E::Enable | E::Cleanup) => Refused,

        (S::DisabledWithoutPassword, E::Enable) => Moves(S::AwaitingPassword),
        (S::DisabledWithoutPassword, E::DeleteByAdmin | E::Cleanup) => Moves(S::Deleted),
        (S::DisabledWithoutPassword, E::DeleteSelf | E::ChangePassword) => {
            Unreachable(NO_SESSION_DISABLED)
        }
        (
            S::DisabledWithoutPassword,
            E::Register | E::ConfirmEmail | E::ResetPassword | E::Disable,
        ) => Refused,

        (S::DisabledWithPassword, E::Enable) => Moves(S::Active),
        (S::DisabledWithPassword, E::DeleteByAdmin) => Moves(S::Deleted),
        (S::DisabledWithPassword, E::DeleteSelf | E::ChangePassword) => {
            Unreachable(NO_SESSION_DISABLED)
        }
        (
            S::DisabledWithPassword,
            E::Register | E::ConfirmEmail | E::ResetPassword | E::Disable | E::Cleanup,
        ) => Refused,
    }
}

impl Labeled for AccountState {
    fn id(self) -> &'static str {
        match self {
            S::New => "new",
            S::AwaitingPassword => "awaiting_password",
            S::Active => "active",
            S::DisabledWithoutPassword => "disabled_without_password",
            S::DisabledWithPassword => "disabled_with_password",
            S::Deleted => "deleted",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::New => "No account",
            S::AwaitingPassword => "Awaiting password",
            S::Active => "Active",
            S::DisabledWithoutPassword => "Disabled (no password)",
            S::DisabledWithPassword => "Disabled (has password)",
            S::Deleted => "Deleted",
        }
    }
}

impl Labeled for AccountEvent {
    fn id(self) -> &'static str {
        match self {
            E::Register => "register",
            E::ConfirmEmail => "confirm_email",
            E::ResetPassword => "reset_password",
            E::ChangePassword => "change_password",
            E::Disable => "disable",
            E::Enable => "enable",
            E::DeleteSelf => "delete_self",
            E::DeleteByAdmin => "delete_by_admin",
            E::Cleanup => "cleanup",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Register => "Register with this email",
            E::ConfirmEmail => "Use a confirmation link",
            E::ResetPassword => "Use a password reset link",
            E::ChangePassword => "Changes own password",
            E::Disable => "Admin disables",
            E::Enable => "Admin enables",
            E::DeleteSelf => "Deletes own account",
            E::DeleteByAdmin => "Admin deletes",
            E::Cleanup => "Hourly cleanup, a week later",
        }
    }
}

#[cfg(test)]
mod tests;
