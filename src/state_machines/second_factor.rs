use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondFactorState {
    SignedOut,
    Waiting,
    SignedIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondFactorEvent {
    EnterPassword,
    RightCode,
    RecoveryCode,
    WrongCode,
    RightCodeLate,
    OpenAccount,
    OpenSignIn,
}

use SecondFactorEvent as E;
use SecondFactorState as S;

pub const SECOND_FACTOR: Machine<SecondFactorState, SecondFactorEvent> = Machine {
    name: "Second-factor sign-in",
    summary: "A browser signing in to an account with two-factor on: after the password it waits for a code, and only a right code signs it in.",
    states: &[S::SignedOut, S::Waiting, S::SignedIn],
    terminal: &[],
    events: &[
        E::EnterPassword,
        E::RightCode,
        E::RecoveryCode,
        E::WrongCode,
        E::RightCodeLate,
        E::OpenAccount,
        E::OpenSignIn,
    ],
    stored_in: &["session:PENDING_SECOND_FACTOR@definitions"],
    rules: &[ACCOUNT_NEEDS_THE_CODE, CODE_WORKS_ONCE, WRONG_CODES_LOCK],
    writers: &[
        Writer {
            caller: "src/two_factor/challenge.rs:begin_second_factor",
            query: "begin_second_factor",
            events: &[E::EnterPassword],
        },
        Writer {
            caller: "src/two_factor/challenge.rs:end_second_factor",
            query: "end_second_factor",
            events: &[
                E::RightCode,
                E::RecoveryCode,
                E::RightCodeLate,
                E::OpenSignIn,
            ],
        },
    ],
};

pub const ACCOUNT_NEEDS_THE_CODE: &str =
    "While waiting for the code, opening the account sends the browser to sign in.";
pub const CODE_WORKS_ONCE: &str =
    "A code from the app that signed in once is refused when entered again.";
pub const WRONG_CODES_LOCK: &str = "After five wrong codes even the right code is refused, however often the password is entered again.";

pub const LATE_MINUTES: i64 = 6;

pub fn expect(from: SecondFactorState, event: SecondFactorEvent) -> Expect<SecondFactorState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (S::SignedIn, _) => Refused,
        (_, E::EnterPassword) => Moves(S::Waiting),
        (S::SignedOut, _) => Refused,
        (S::Waiting, E::RightCode | E::RecoveryCode) => Moves(S::SignedIn),
        (S::Waiting, E::RightCodeLate | E::OpenSignIn) => Moves(S::SignedOut),
        (S::Waiting, E::WrongCode | E::OpenAccount) => Refused,
    }
}

impl Labeled for SecondFactorState {
    fn id(self) -> &'static str {
        match self {
            S::SignedOut => "signed_out",
            S::Waiting => "waiting",
            S::SignedIn => "signed_in",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::SignedOut => "Signed out",
            S::Waiting => "Waiting for the code",
            S::SignedIn => "Signed in",
        }
    }
}

impl Labeled for SecondFactorEvent {
    fn id(self) -> &'static str {
        match self {
            E::EnterPassword => "enter_password",
            E::RightCode => "right_code",
            E::RecoveryCode => "recovery_code",
            E::WrongCode => "wrong_code",
            E::RightCodeLate => "right_code_late",
            E::OpenAccount => "open_account",
            E::OpenSignIn => "open_sign_in",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::EnterPassword => "Enters the right password",
            E::RightCode => "Enters the code from the app",
            E::RecoveryCode => "Enters a recovery code",
            E::WrongCode => "Enters a wrong code",
            E::RightCodeLate => "Enters the code from the app 6 minutes later",
            E::OpenAccount => "Opens the account",
            E::OpenSignIn => "Opens the sign-in page",
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits {
    use super::*;
    use crate::two_factor::challenge::PENDING_WINDOW_SECONDS;

    #[test]
    fn the_late_wait_in_the_label_matches_the_limit() {
        const { assert!(LATE_MINUTES * 60 > PENDING_WINDOW_SECONDS) };
        assert!(
            E::RightCodeLate
                .label()
                .contains(&format!("{LATE_MINUTES} minutes"))
        );
    }
}
