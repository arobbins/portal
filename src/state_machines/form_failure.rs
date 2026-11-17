use super::{Expect, Labeled, Machine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureState {
    Empty,
    Problem,
    TooMany,
    SignedIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureEvent {
    WrongPassword,
    UnknownEmail,
    RightPassword,
    Reload,
    FiveWrong,
}

use FailureEvent as E;
use FailureState as S;

pub const FORM_FAILURE: Machine<FailureState, FailureEvent> = Machine {
    name: "Form failure",
    summary: "What a form shows after it's sent back with a problem, using the sign-in form: every form re-renders its problem the same way.",
    states: &[S::Empty, S::Problem, S::TooMany, S::SignedIn],
    terminal: &[S::SignedIn],
    events: &[
        E::WrongPassword,
        E::UnknownEmail,
        E::RightPassword,
        E::Reload,
        E::FiveWrong,
    ],
    stored_in: &[],
    rules: &[
        PROBLEMS_ARE_NOT_STICKY,
        FIELDS_POINT_AT_THE_PROBLEM,
        STATUS_MATCHES_THE_PROBLEM,
        NOTHING_IS_REVEALED,
    ],
    writers: &[],
};

pub const PROBLEMS_ARE_NOT_STICKY: &str =
    "A problem shows only on the answer to that submit: opening the form again shows it clean.";
pub const FIELDS_POINT_AT_THE_PROBLEM: &str =
    "The fields a problem is about are marked invalid and point screen readers at the message.";
pub const STATUS_MATCHES_THE_PROBLEM: &str =
    "A problem the person can fix answers 200; too many attempts answers 429.";
pub const NOTHING_IS_REVEALED: &str =
    "A wrong password and an unknown email get exactly the same message.";

const SIGNED_IN: &str = "Signed-in people are sent away from the sign-in form.";

pub fn expect(from: FailureState, event: FailureEvent) -> Expect<FailureState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::SignedIn, _) => Unreachable(SIGNED_IN),
        (S::TooMany, E::WrongPassword | E::RightPassword | E::FiveWrong) => Refused,
        (_, E::WrongPassword | E::UnknownEmail) => Moves(S::Problem),
        (_, E::RightPassword) => Moves(S::SignedIn),
        (_, E::FiveWrong) => Moves(S::TooMany),
        (S::Empty, E::Reload) => Refused,
        (_, E::Reload) => Moves(S::Empty),
    }
}

impl Labeled for FailureState {
    fn id(self) -> &'static str {
        match self {
            S::Empty => "empty",
            S::Problem => "problem",
            S::TooMany => "too_many",
            S::SignedIn => "signed_in",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Empty => "Clean form",
            S::Problem => "Showing a problem",
            S::TooMany => "Showing too many attempts",
            S::SignedIn => "Signed in",
        }
    }
}

impl Labeled for FailureEvent {
    fn id(self) -> &'static str {
        match self {
            E::WrongPassword => "wrong_password",
            E::UnknownEmail => "unknown_email",
            E::RightPassword => "right_password",
            E::Reload => "reload",
            E::FiveWrong => "five_wrong",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::WrongPassword => "Sends a wrong password",
            E::UnknownEmail => "Sends an email with no account",
            E::RightPassword => "Sends the right password",
            E::Reload => "Opens the form again",
            E::FiveWrong => "Sends five wrong passwords",
        }
    }
}

#[cfg(test)]
mod tests;
