use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestState {
    New,
    Pending,
    Approved,
    Dead,
    Gone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestEvent {
    Start,
    StartTooOften,
    ReturnsAfterEightMinutes,
    Approve,
    Deny,
    ExchangeCode,
    WaitElevenMinutes,
    Cleanup,
}

use RequestEvent as E;
use RequestState as S;

pub const AUTHORIZATION_REQUEST: Machine<RequestState, RequestEvent> = Machine {
    name: "Authorization request",
    summary: "One app's request to connect to an account: started by the app, approved or denied by the signed-in owner, and used once for its code.",
    states: &[S::New, S::Pending, S::Approved, S::Dead, S::Gone],
    terminal: &[S::Gone],
    events: &[
        E::Start,
        E::StartTooOften,
        E::ReturnsAfterEightMinutes,
        E::Approve,
        E::Deny,
        E::ExchangeCode,
        E::WaitElevenMinutes,
        E::Cleanup,
    ],
    stored_in: &[
        "oauth_requests (row)",
        "oauth_requests.status",
        "oauth_requests.acted_at",
    ],
    rules: &[
        KEPT_ALIVE,
        ANSWERS_GO_BACK_TO_THE_APP,
        CODE_ONCE_AND_HASHED,
        OVER_THE_LIMIT_STORES_NOTHING,
    ],
    writers: &[
        writer(
            "src/oauth/authorize.rs:store_request",
            "query_insert_request",
            &[E::Start],
        ),
        writer(
            "src/oauth/authorize.rs:keep_request_alive",
            "query_touch_request",
            &[E::ReturnsAfterEightMinutes],
        ),
        writer(
            "src/oauth/actions/connect.rs:connect_app",
            "query_approve_request",
            &[E::Approve],
        ),
        writer(
            "src/oauth/actions/connect.rs:deny_app",
            "query_delete_pending_request",
            &[E::Deny],
        ),
        writer(
            "src/oauth/grant.rs:exchange_code",
            "query_consume_code",
            &[E::ExchangeCode],
        ),
        writer(
            "src/oauth/lifecycle.rs:delete_expired",
            "query_delete_stale_requests",
            &[E::Cleanup],
        ),
    ],
};

pub const KEPT_ALIVE: &str =
    "Opening the approval screen keeps a pending request alive for another 10 minutes.";
pub const ANSWERS_GO_BACK_TO_THE_APP: &str =
    "Approving sends the app back a code and its state; denying sends it back access_denied.";
pub const OVER_THE_LIMIT_STORES_NOTHING: &str = "Starting more than 30 times an hour from one address gets the 429 \"Too many sign-in requests\" page and stores no request.";
pub const CODE_ONCE_AND_HASHED: &str =
    "The code is stored only as a hash, works once, and only within 10 minutes of approval.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [RequestEvent],
) -> Writer<RequestEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: RequestState, event: RequestEvent) -> Expect<RequestState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (S::New, E::Start) => Moves(S::Pending),
        (S::Pending, E::Approve) => Moves(S::Approved),
        (S::Pending, E::Deny) | (S::Approved, E::ExchangeCode) | (S::Dead, E::Cleanup) => {
            Moves(S::Gone)
        }
        (S::Pending | S::Approved, E::WaitElevenMinutes) => Moves(S::Dead),
        _ => Refused,
    }
}

impl Labeled for RequestState {
    fn id(self) -> &'static str {
        match self {
            S::New => "new",
            S::Pending => "pending",
            S::Approved => "approved",
            S::Dead => "dead",
            S::Gone => "gone",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::New => "Not started",
            S::Pending => "Waiting for the owner",
            S::Approved => "Approved, code issued",
            S::Dead => "Idle 10 minutes",
            S::Gone => "Gone",
        }
    }
}

impl Labeled for RequestEvent {
    fn id(self) -> &'static str {
        match self {
            E::Start => "start",
            E::StartTooOften => "start_too_often",
            E::ReturnsAfterEightMinutes => "returns_after_eight_minutes",
            E::Approve => "approve",
            E::Deny => "deny",
            E::ExchangeCode => "exchange_code",
            E::WaitElevenMinutes => "wait_eleven_minutes",
            E::Cleanup => "cleanup",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Start => "The app opens /oauth/authorize",
            E::StartTooOften => "The app opens /oauth/authorize a 31st time within an hour",
            E::ReturnsAfterEightMinutes => "Owner reopens the approval screen 8 minutes later",
            E::Approve => "Owner approves",
            E::Deny => "Owner denies",
            E::ExchangeCode => "The app exchanges the code",
            E::WaitElevenMinutes => "11 idle minutes pass",
            E::Cleanup => "Hourly cleanup",
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits {
    use super::*;
    use crate::oauth::secrets::{CODE_LIFETIME, REQUEST_IDLE_LIMIT};
    use crate::rate_limit::AUTHORIZATION_STARTS_PER_IP_PER_HOUR;

    #[test]
    fn the_start_limit_in_the_labels_matches_the_limit() {
        let limit = AUTHORIZATION_STARTS_PER_IP_PER_HOUR;
        assert!(OVER_THE_LIMIT_STORES_NOTHING.contains(&format!("more than {limit} times")));
        let label = E::StartTooOften.label();
        assert!(label.contains(&format!("a {}st time", limit + 1)));
    }

    #[test]
    fn the_waits_in_the_labels_match_the_limit() {
        let limit = REQUEST_IDLE_LIMIT.whole_minutes();
        assert_eq!(CODE_LIFETIME, REQUEST_IDLE_LIMIT);
        assert!(KEPT_ALIVE.contains(&format!("{limit} minutes")));
        assert!(CODE_ONCE_AND_HASHED.contains(&format!("{limit} minutes")));
        assert!(S::Dead.label().contains(&format!("{limit} minutes")));
        assert!(8 < limit && limit < 11);
        assert!(E::ReturnsAfterEightMinutes.label().contains("8 minutes"));
        assert!(E::WaitElevenMinutes.label().contains("11 idle minutes"));
    }
}
