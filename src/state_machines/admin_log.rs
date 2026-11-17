use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogState {
    Nothing,
    Logged,
    AccountGone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEvent {
    AdminActs,
    AdminChangesNothing,
    ConsoleGrantsAdmin,
    AdminDeletesAccount,
    ActingAdminLeaves,
    CleanupYearsLater,
}

use LogEvent as E;
use LogState as S;

pub const ADMIN_LOG: Machine<LogState, LogEvent> = Machine {
    name: "Admin log",
    summary: "What the admin log holds about one account: an entry for every admin action and role change, kept forever, even after the account or the admin is deleted.",
    states: &[S::Nothing, S::Logged, S::AccountGone],
    terminal: &[],
    events: &[
        E::AdminActs,
        E::AdminChangesNothing,
        E::ConsoleGrantsAdmin,
        E::AdminDeletesAccount,
        E::ActingAdminLeaves,
        E::CleanupYearsLater,
    ],
    stored_in: &["admin_log (row)", "admin_log.*"],
    rules: &[
        NEVER_EDITED,
        NEVER_REMOVED,
        NAMES_WHO_DID_IT,
        SHOWN_ON_THE_LOG_PAGE,
    ],
    writers: &[Writer {
        caller: "src/admin/log.rs:log_admin_action",
        query: "query_insert_admin_log",
        events: &[E::AdminActs, E::ConsoleGrantsAdmin, E::AdminDeletesAccount],
    }],
};

pub const NEVER_EDITED: &str = "An entry never changes after it's written.";
pub const NEVER_REMOVED: &str =
    "No entry is ever removed: not by cleanup, nor when the account or the admin is deleted.";
pub const NAMES_WHO_DID_IT: &str =
    "A new entry names the admin who acted, or the server console for role changes.";
pub const SHOWN_ON_THE_LOG_PAGE: &str =
    "The log page lists every entry, linking the account only while it exists.";

const GONE: &str = "The account is gone, so nothing can be done to it.";

pub fn expect(from: LogState, event: LogEvent) -> Expect<LogState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::AccountGone, E::ActingAdminLeaves | E::CleanupYearsLater) => Refused,
        (S::AccountGone, _) => Unreachable(GONE),
        (_, E::AdminActs | E::ConsoleGrantsAdmin) => Moves(S::Logged),
        (_, E::AdminDeletesAccount) => Moves(S::AccountGone),
        (_, E::AdminChangesNothing | E::ActingAdminLeaves | E::CleanupYearsLater) => Refused,
    }
}

impl Labeled for LogState {
    fn id(self) -> &'static str {
        match self {
            S::Nothing => "nothing",
            S::Logged => "logged",
            S::AccountGone => "account_gone",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Nothing => "Nothing logged",
            S::Logged => "Entries logged",
            S::AccountGone => "Entries, account deleted",
        }
    }
}

impl Labeled for LogEvent {
    fn id(self) -> &'static str {
        match self {
            E::AdminActs => "admin_acts",
            E::AdminChangesNothing => "admin_changes_nothing",
            E::ConsoleGrantsAdmin => "console_grants_admin",
            E::AdminDeletesAccount => "admin_deletes_account",
            E::ActingAdminLeaves => "acting_admin_leaves",
            E::CleanupYearsLater => "cleanup_years_later",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::AdminActs => "An admin signs them out everywhere",
            E::AdminChangesNothing => "An admin enables an active account",
            E::ConsoleGrantsAdmin => "The server console makes them an admin",
            E::AdminDeletesAccount => "An admin deletes the account",
            E::ActingAdminLeaves => "The admin who acted deletes their own account",
            E::CleanupYearsLater => "Every cleanup, years later",
        }
    }
}

#[cfg(test)]
mod tests;
