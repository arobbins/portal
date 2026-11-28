use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    Clear,
    Failing,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockEvent {
    WrongPassword,
    RightPassword,
    WindowPasses,
    WrongFromAnotherIp,
    WrongForAnotherAccount,
}

use LockEvent as E;
use LockState as S;

pub const RATE_LIMIT: Machine<LockState, LockEvent> = Machine {
    name: "Rate limit",
    summary: "The sign-in lockout for one account from one IP address: wrong passwords build up over a sliding 15-minute window, and five lock it.",
    states: &[S::Clear, S::Failing, S::Locked],
    terminal: &[],
    events: &[
        E::WrongPassword,
        E::RightPassword,
        E::WindowPasses,
        E::WrongFromAnotherIp,
        E::WrongForAnotherAccount,
    ],
    stored_in: &["rate_limit_hits (row)"],
    rules: &[
        LOCKED_EVEN_WITH_THE_RIGHT_PASSWORD,
        ONLY_THIS_ADDRESS_IS_LOCKED,
        LIMITER_AGREES,
    ],
    writers: &[
        writer(
            "src/rate_limit.rs:record",
            "query_insert_rate_limit_hit",
            &[],
        ),
        writer(
            "src/rate_limit.rs:take_slot",
            "query_take_rate_limit_slot",
            &[E::WrongPassword, E::RightPassword],
        ),
        writer(
            "src/rate_limit.rs:clear_failed_logins",
            "query_delete_rate_limit_hits",
            &[E::RightPassword],
        ),
        writer(
            "src/rate_limit.rs:clear_failed_logins",
            "query_delete_newest_rate_limit_hit",
            &[E::RightPassword],
        ),
        writer(
            "src/rate_limit.rs:clear_failed_second_factors",
            "query_delete_rate_limit_hits",
            &[],
        ),
        writer(
            "src/rate_limit.rs:forget_old_hits",
            "query_delete_old_rate_limit_hits",
            &[],
        ),
    ],
};

pub const LOCKED_EVEN_WITH_THE_RIGHT_PASSWORD: &str = "Below five failures the right password signs in; once locked, even the right password is refused with \"Too many attempts\".";
pub const LIMITER_AGREES: &str =
    "The limiter itself reports the account locked exactly when five failures sit in the window.";
pub const ONLY_THIS_ADDRESS_IS_LOCKED: &str =
    "A lockout from one IP address doesn't stop the owner signing in from another.";

pub const FAILURES_TO_LOCK: i64 = 5;
pub const WINDOW_MINUTES: i64 = 15;

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [LockEvent],
) -> Writer<LockEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: LockState, event: LockEvent) -> Expect<LockState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (S::Clear, E::WrongPassword) => Moves(S::Failing),
        (S::Failing, E::WrongPassword) => Moves(S::Locked),
        (S::Failing, E::RightPassword) | (S::Failing | S::Locked, E::WindowPasses) => {
            Moves(S::Clear)
        }
        _ => Refused,
    }
}

impl Labeled for LockState {
    fn id(self) -> &'static str {
        match self {
            S::Clear => "clear",
            S::Failing => "failing",
            S::Locked => "locked",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Clear => "No failures",
            S::Failing => "Four failures",
            S::Locked => "Locked",
        }
    }
}

impl Labeled for LockEvent {
    fn id(self) -> &'static str {
        match self {
            E::WrongPassword => "wrong_password",
            E::RightPassword => "right_password",
            E::WindowPasses => "window_passes",
            E::WrongFromAnotherIp => "wrong_from_another_ip",
            E::WrongForAnotherAccount => "wrong_for_another_account",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::WrongPassword => "A wrong password",
            E::RightPassword => "The right password",
            E::WindowPasses => "The window slides past the failures",
            E::WrongFromAnotherIp => "A wrong password from another IP",
            E::WrongForAnotherAccount => "A wrong password for another account",
        }
    }
}

#[cfg(test)]
mod tests;
