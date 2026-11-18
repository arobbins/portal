use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownState {
    Unknown,
    Known,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownEvent {
    SignIn,
    SignInWithAlertsOff,
    ResetFromEmail,
    ClearCookies,
    UnusedFor400Days,
}

use KnownEvent as E;
use KnownState as S;

const RECOGNIZE: &str = "src/devices/known.rs:recognize_device";

pub const KNOWN_DEVICE: Machine<KnownState, KnownEvent> = Machine {
    name: "Known device",
    summary: "Whether this browser is one the account has signed in from before, which decides if a sign-in sends a \"New sign-in\" email.",
    states: &[S::Unknown, S::Known],
    terminal: &[],
    events: &[
        E::SignIn,
        E::SignInWithAlertsOff,
        E::ResetFromEmail,
        E::ClearCookies,
        E::UnusedFor400Days,
    ],
    stored_in: &["known_devices", "known_devices.*"],
    rules: &[ALERT_ONLY_WHEN_NEW, ONLY_THE_HASH],
    writers: &[
        Writer {
            caller: RECOGNIZE,
            query: "query_touch_known_device",
            events: &[E::SignIn, E::SignInWithAlertsOff, E::ResetFromEmail],
        },
        Writer {
            caller: RECOGNIZE,
            query: "query_insert_known_device",
            events: &[E::SignIn, E::SignInWithAlertsOff, E::ResetFromEmail],
        },
        Writer {
            caller: "src/devices/known.rs:forget_unseen_devices",
            query: "query_delete_stale_known_devices",
            events: &[E::UnusedFor400Days],
        },
    ],
};

pub const ALERT_ONLY_WHEN_NEW: &str = "Exactly one \"New sign-in\" email is queued when a password sign-in makes the browser known while alerts are on, and none otherwise.";
pub const ONLY_THE_HASH: &str =
    "The database keeps only a hash of the browser's cookie, never the cookie itself.";

pub const UNUSED_DAYS: i64 = 401;

pub fn expect(from: KnownState, event: KnownEvent) -> Expect<KnownState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (_, E::SignIn | E::SignInWithAlertsOff | E::ResetFromEmail) => Moves(S::Known),
        (S::Unknown, E::ClearCookies | E::UnusedFor400Days) => Refused,
        (S::Known, E::ClearCookies | E::UnusedFor400Days) => Moves(S::Unknown),
    }
}

impl Labeled for KnownState {
    fn id(self) -> &'static str {
        match self {
            S::Unknown => "unknown",
            S::Known => "known",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Unknown => "Unknown browser",
            S::Known => "Known browser",
        }
    }
}

impl Labeled for KnownEvent {
    fn id(self) -> &'static str {
        match self {
            E::SignIn => "sign_in",
            E::SignInWithAlertsOff => "sign_in_with_alerts_off",
            E::ResetFromEmail => "reset_from_email",
            E::ClearCookies => "clear_cookies",
            E::UnusedFor400Days => "unused_for_400_days",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::SignIn => "Signs in with the password",
            E::SignInWithAlertsOff => "Turns sign-in alerts off, then signs in",
            E::ResetFromEmail => "Resets the password from an emailed link",
            E::ClearCookies => "Clears the browser's cookies",
            E::UnusedFor400Days => "Not seen for 401 days, then cleanup runs",
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits {
    use super::*;
    use crate::devices::known::KNOWN_DEVICE_LIFETIME;

    #[test]
    fn the_unused_wait_in_the_label_matches_the_lifetime() {
        assert!(UNUSED_DAYS > KNOWN_DEVICE_LIFETIME.whole_days());
        assert!(
            E::UnusedFor400Days
                .label()
                .contains(&format!("{UNUSED_DAYS} days"))
        );
    }
}
