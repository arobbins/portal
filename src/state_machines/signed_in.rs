use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedInState {
    SignedOut,
    SignedIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedInEvent {
    SignIn,
    SignInWrongPassword,
    SignOut,
    ChangePassword,
    OtherChangesPassword,
    OtherSignsThisOut,
    OtherSignsOutOthers,
    PasswordReset,
    EmailChangeConfirmed,
    AdminSignsOutEverywhere,
    AdminDisables,
    IdleADay,
}

use SignedInEvent as E;
use SignedInState as S;

pub const SIGNED_IN: Machine<SignedInState, SignedInEvent> = Machine {
    name: "Signed in",
    summary: "Whether this browser is signed in, while the same person is also signed in on another device.",
    states: &[S::SignedOut, S::SignedIn],
    terminal: &[],
    events: &[
        E::SignIn,
        E::SignInWrongPassword,
        E::SignOut,
        E::ChangePassword,
        E::OtherChangesPassword,
        E::OtherSignsThisOut,
        E::OtherSignsOutOthers,
        E::PasswordReset,
        E::EmailChangeConfirmed,
        E::AdminSignsOutEverywhere,
        E::AdminDisables,
        E::IdleADay,
    ],
    stored_in: &[
        "user_devices (row)",
        "session:SESSION_AUTH",
        "users.session_version",
    ],
    rules: &[
        ONE_ROW_PER_BROWSER,
        OLD_COOKIE_IS_DEAD,
        OTHER_DEVICE_ONLY_WHEN_MEANT,
        SIGN_INS_ARE_RECORDED,
    ],
    writers: &[
        writer(START, "remember", &[E::SignIn]),
        writer(START_DEVICE, "query_insert_device", &[E::SignIn]),
        writer(STAY, "remember", &[E::ChangePassword]),
        writer(END_DEVICE, "query_delete_device", &[E::SignOut]),
        writer(
            CHANGE,
            "query_update_password",
            &[E::ChangePassword, E::OtherChangesPassword],
        ),
        writer(DEVICE_OUT, "query_delete_device", &[E::OtherSignsThisOut]),
        writer(
            OTHERS_OUT,
            "query_bump_session_version",
            &[E::OtherSignsOutOthers],
        ),
        writer(
            OTHERS_OUT,
            "query_delete_other_devices",
            &[E::OtherSignsOutOthers],
        ),
        writer(LINK, "query_update_password", &[E::PasswordReset]),
        writer(LINK, "query_set_password_and_verify_user", &[]),
        writer(EMAIL, "query_update_user_email", &[E::EmailChangeConfirmed]),
        writer(
            ADMIN_SIGN_OUT,
            "query_bump_session_version",
            &[E::AdminSignsOutEverywhere],
        ),
        writer(ADMIN_DISABLE, "query_disable_user", &[E::AdminDisables]),
        writer(
            EVERY_DEVICE,
            "query_delete_user_devices",
            &[E::AdminSignsOutEverywhere, E::AdminDisables],
        ),
        writer(STALE, "query_delete_stale_devices", &[E::IdleADay]),
        writer("src/seed.rs:run", "query_set_password_and_verify_user", &[]),
    ],
};

const START: &str = "src/auth/session.rs:start_session";
const STAY: &str = "src/auth/session.rs:stay_signed_in";
const START_DEVICE: &str = "src/devices/lifecycle.rs:start_device";
const END_DEVICE: &str = "src/devices/lifecycle.rs:end_device";
const EVERY_DEVICE: &str = "src/devices/lifecycle.rs:end_every_device";
const STALE: &str = "src/devices/lifecycle.rs:delete_stale_devices";
const DEVICE_OUT: &str = "src/devices/actions.rs:sign_out_device";
const OTHERS_OUT: &str = "src/devices/actions.rs:sign_out_other_devices";
const CHANGE: &str = "src/auth/actions/change_password.rs:change_password";
const LINK: &str = "src/auth/actions/link_password.rs:apply_password";
const EMAIL: &str = "src/auth/actions/confirm_email.rs:switch_email";
const ADMIN_SIGN_OUT: &str = "src/admin/actions/manage.rs:sign_out_user";
const ADMIN_DISABLE: &str = "src/admin/actions/manage.rs:disable_user";

pub const ONE_ROW_PER_BROWSER: &str = "Every signed-in browser has exactly one live device row (on the current session version), and a signed-out one has none; stale rows are only ever deleted.";
pub const OLD_COOKIE_IS_DEAD: &str =
    "Once a browser is signed out, its old session cookie never signs anyone back in.";
pub const OTHER_DEVICE_ONLY_WHEN_MEANT: &str = "The other device is signed out only when an event meant to reach every device succeeds: a password change or reset, an email change, an admin, or a day idle.";
pub const SIGN_INS_ARE_RECORDED: &str =
    "Every successful sign-in records one signed_in event; nothing else does.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [SignedInEvent],
) -> Writer<SignedInEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: SignedInState, event: SignedInEvent) -> Expect<SignedInState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (_, E::SignIn) => Moves(S::SignedIn),
        (S::SignedOut, _) | (S::SignedIn, E::SignInWrongPassword) => Refused,
        (S::SignedIn, E::ChangePassword) => Moves(S::SignedIn),
        (S::SignedIn, _) => Moves(S::SignedOut),
    }
}

pub fn other_device_stays(event: SignedInEvent) -> bool {
    matches!(
        event,
        E::SignIn
            | E::SignInWrongPassword
            | E::SignOut
            | E::OtherChangesPassword
            | E::OtherSignsThisOut
            | E::OtherSignsOutOthers
    )
}

impl Labeled for SignedInState {
    fn id(self) -> &'static str {
        match self {
            S::SignedOut => "signed_out",
            S::SignedIn => "signed_in",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::SignedOut => "Signed out",
            S::SignedIn => "Signed in",
        }
    }
}

impl Labeled for SignedInEvent {
    fn id(self) -> &'static str {
        match self {
            E::SignIn => "sign_in",
            E::SignInWrongPassword => "sign_in_wrong_password",
            E::SignOut => "sign_out",
            E::ChangePassword => "change_password",
            E::OtherChangesPassword => "other_changes_password",
            E::OtherSignsThisOut => "other_signs_this_out",
            E::OtherSignsOutOthers => "other_signs_out_others",
            E::PasswordReset => "password_reset",
            E::EmailChangeConfirmed => "email_change_confirmed",
            E::AdminSignsOutEverywhere => "admin_signs_out_everywhere",
            E::AdminDisables => "admin_disables",
            E::IdleADay => "idle_a_day",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::SignIn => "Signs in with the password",
            E::SignInWrongPassword => "Signs in with a wrong password",
            E::SignOut => "Signs out",
            E::ChangePassword => "Changes the password here",
            E::OtherChangesPassword => "Other device changes the password",
            E::OtherSignsThisOut => "Other device signs this one out",
            E::OtherSignsOutOthers => "Other device signs out all others",
            E::PasswordReset => "Password reset by email link",
            E::EmailChangeConfirmed => "Email change confirmed",
            E::AdminSignsOutEverywhere => "Admin signs them out everywhere",
            E::AdminDisables => "Admin disables the account",
            E::IdleADay => "Both idle for a day, then cleanup",
        }
    }
}

#[cfg(test)]
mod tests;
