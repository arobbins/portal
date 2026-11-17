use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantState {
    None,
    Fresh,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantEvent {
    SignInWhenDue,
    SignInWhenNotDue,
    PickNotNow,
    StartWithoutPassword,
    WaitSixMinutes,
    ResetPassword,
    SignOut,
    SignInAsSomeoneElse,
}

use GrantEvent as E;
use GrantState as S;

pub const PASSKEY_PROMPT_GRANT: Machine<GrantState, GrantEvent> = Machine {
    name: "Passkey prompt grant",
    summary: "The short-lived permission, left by a password sign-in that showed the passkey prompt, to add a passkey without typing the password again.",
    states: &[S::None, S::Fresh, S::Stale],
    terminal: &[],
    events: &[
        E::SignInWhenDue,
        E::SignInWhenNotDue,
        E::PickNotNow,
        E::StartWithoutPassword,
        E::WaitSixMinutes,
        E::ResetPassword,
        E::SignOut,
        E::SignInAsSomeoneElse,
    ],
    stored_in: &["session:PASSKEY_PROMPT_GRANT"],
    rules: &[SKIPS_PASSWORD_ONLY_WHEN_FRESH, PAGE_ONLY_WITH_A_FRESH_GRANT],
    writers: &[
        Writer {
            caller: "src/passkeys/pages/passkey_setup.rs:after_password_sign_in",
            query: "grant_passkey_prompt",
            events: &[E::SignInWhenDue],
        },
        Writer {
            caller: "src/auth/session.rs:start_session",
            query: "revoke_passkey_prompt",
            events: &[
                E::SignInWhenDue,
                E::SignInWhenNotDue,
                E::ResetPassword,
                E::SignInAsSomeoneElse,
            ],
        },
        Writer {
            caller: "src/passkeys/session.rs:revoke_prompt",
            query: "revoke_passkey_prompt",
            events: &[E::PickNotNow],
        },
    ],
};

pub const SKIPS_PASSWORD_ONLY_WHEN_FRESH: &str =
    "Adding a passkey without the password works only with a fresh grant.";
pub const PAGE_ONLY_WITH_A_FRESH_GRANT: &str =
    "The prompt page opens only with a fresh grant while the prompt is due.";

pub fn expect(from: GrantState, event: GrantEvent) -> Expect<GrantState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (_, E::SignInWhenDue) => Moves(S::Fresh),
        (S::Fresh, E::WaitSixMinutes) => Moves(S::Stale),
        (_, E::StartWithoutPassword | E::WaitSixMinutes) => Refused,
        (S::Stale, E::PickNotNow) | (S::None, _) => Refused,
        _ => Moves(S::None),
    }
}

impl Labeled for GrantState {
    fn id(self) -> &'static str {
        match self {
            S::None => "none",
            S::Fresh => "fresh",
            S::Stale => "stale",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::None => "No grant",
            S::Fresh => "Fresh grant",
            S::Stale => "Stale grant",
        }
    }
}

impl Labeled for GrantEvent {
    fn id(self) -> &'static str {
        match self {
            E::SignInWhenDue => "sign_in_when_due",
            E::SignInWhenNotDue => "sign_in_when_not_due",
            E::PickNotNow => "pick_not_now",
            E::StartWithoutPassword => "start_without_password",
            E::WaitSixMinutes => "wait_six_minutes",
            E::ResetPassword => "reset_password",
            E::SignOut => "sign_out",
            E::SignInAsSomeoneElse => "sign_in_as_someone_else",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::SignInWhenDue => "Signs in while the prompt is due",
            E::SignInWhenNotDue => "Signs in after declining the prompt",
            E::PickNotNow => "Picks \"Not now\"",
            E::StartWithoutPassword => "Starts adding a passkey without the password",
            E::WaitSixMinutes => "Waits six minutes",
            E::ResetPassword => "Signs in through a reset link",
            E::SignOut => "Signs out",
            E::SignInAsSomeoneElse => "Signs in as someone else here",
        }
    }
}

#[cfg(test)]
mod tests;
