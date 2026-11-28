use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventState {
    None,
    Recorded,
    PastRetention,
    Gone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventEvent {
    ChangeSucceeds,
    ChangeFails,
    OwnerReadsActivity,
    ThirteenMonthsPass,
    Cleanup,
    AccountDeleted,
}

use EventEvent as E;
use EventState as S;

pub const ACCOUNT_EVENT: Machine<EventState, EventEvent> = Machine {
    name: "Account event",
    summary: "One entry in an account's activity: written with the change it describes, never edited, removed by cleanup after 13 months or with the account.",
    states: &[S::None, S::Recorded, S::PastRetention, S::Gone],
    terminal: &[S::Gone],
    events: &[
        E::ChangeSucceeds,
        E::ChangeFails,
        E::OwnerReadsActivity,
        E::ThirteenMonthsPass,
        E::Cleanup,
        E::AccountDeleted,
    ],
    stored_in: &["account_events (row)", "account_events.*"],
    rules: &[NEVER_EDITED, EXACTLY_WITH_ITS_CHANGE, SHOWN_TO_THE_OWNER],
    writers: &[
        Writer {
            caller: "src/user/events.rs:record_event",
            query: "query_insert_account_event",
            events: &[E::ChangeSucceeds],
        },
        Writer {
            caller: "src/admin/log.rs:log_admin_action",
            query: "query_insert_account_event",
            events: &[E::ChangeSucceeds],
        },
        Writer {
            caller: "src/user/events.rs:delete_old_events",
            query: "query_delete_old_account_events",
            events: &[E::Cleanup],
        },
    ],
};

pub const NEVER_EDITED: &str = "An entry never changes after it's written.";
pub const EXACTLY_WITH_ITS_CHANGE: &str =
    "An entry is written exactly when its change happens: a failed change leaves none.";
pub const SHOWN_TO_THE_OWNER: &str = "The owner's activity list shows the entry while it exists.";

const GONE: &str = "The entry is gone.";

pub fn expect(from: EventState, event: EventEvent) -> Expect<EventState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::Gone, _) => Unreachable(GONE),
        (S::None, E::ChangeSucceeds) => Moves(S::Recorded),
        (S::Recorded, E::ThirteenMonthsPass) => Moves(S::PastRetention),
        (S::PastRetention, E::Cleanup) => Moves(S::Gone),
        (S::Recorded | S::PastRetention, E::AccountDeleted) => Moves(S::Gone),
        _ => Refused,
    }
}

impl Labeled for EventState {
    fn id(self) -> &'static str {
        match self {
            S::None => "none",
            S::Recorded => "recorded",
            S::PastRetention => "past_retention",
            S::Gone => "gone",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::None => "Not written",
            S::Recorded => "Recorded",
            S::PastRetention => "Older than 13 months",
            S::Gone => "Removed",
        }
    }
}

impl Labeled for EventEvent {
    fn id(self) -> &'static str {
        match self {
            E::ChangeSucceeds => "change_succeeds",
            E::ChangeFails => "change_fails",
            E::OwnerReadsActivity => "owner_reads_activity",
            E::ThirteenMonthsPass => "thirteen_months_pass",
            E::Cleanup => "cleanup",
            E::AccountDeleted => "account_deleted",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::ChangeSucceeds => "Renames themselves",
            E::ChangeFails => "Tries a blank name",
            E::OwnerReadsActivity => "Opens their activity",
            E::ThirteenMonthsPass => "Thirteen months pass",
            E::Cleanup => "Hourly cleanup",
            E::AccountDeleted => "Deletes the account",
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits {
    use super::*;
    use crate::user::events::EVENT_RETENTION;

    #[test]
    fn the_retention_in_the_labels_matches_the_limit() {
        assert_eq!(EVENT_RETENTION.whole_days(), 395);
        assert!(S::PastRetention.label().contains("13 months"));
        assert!(ACCOUNT_EVENT.summary.contains("13 months"));
    }
}
