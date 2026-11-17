use super::{Expect, Labeled, Machine};
use crate::account::tabs::AccountTab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    Shows(AccountTab),
    NotFound,
    SignedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabEvent {
    Open(AccountTab),
    OpenAccount,
    OpenUnknown,
    SignOut,
}

use AccountTab as T;
use TabEvent as E;
use TabState as S;

pub const ACCOUNT_TAB: Machine<TabState, TabEvent> = Machine {
    name: "Account tab",
    summary: "Which account settings tab a signed-in browser is looking at, chosen only by the URL.",
    states: &[
        S::Shows(T::Profile),
        S::Shows(T::Security),
        S::Shows(T::Billing),
        S::Shows(T::Settings),
        S::Shows(T::Developer),
        S::NotFound,
        S::SignedOut,
    ],
    terminal: &[],
    events: &[
        E::Open(T::Profile),
        E::Open(T::Security),
        E::Open(T::Billing),
        E::Open(T::Settings),
        E::Open(T::Developer),
        E::OpenAccount,
        E::OpenUnknown,
        E::SignOut,
    ],
    stored_in: &[],
    rules: &[
        ONE_CURRENT_TAB,
        TITLE_NAMES_THE_TAB,
        SECTIONS_ARE_ON_THEIR_TAB,
    ],
    writers: &[],
};

pub const ONE_CURRENT_TAB: &str =
    "Exactly one tab is marked as the current page, and it's the tab being shown.";
pub const TITLE_NAMES_THE_TAB: &str = "The page title names the tab being shown.";
pub const SECTIONS_ARE_ON_THEIR_TAB: &str =
    "Every section the tab owns has its anchor on the page, so links to it land.";

const NEEDS_SIGN_IN: &str = "Signing in again isn't part of this machine.";

pub fn expect(from: TabState, event: TabEvent) -> Expect<TabState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::SignedOut, E::SignOut) => Unreachable(NEEDS_SIGN_IN),
        (S::SignedOut, _) => Refused,
        (_, E::Open(tab)) => Moves(S::Shows(tab)),
        (_, E::OpenAccount) => Moves(S::Shows(T::Profile)),
        (_, E::OpenUnknown) => Moves(S::NotFound),
        (_, E::SignOut) => Moves(S::SignedOut),
    }
}

fn tab_id(tab: AccountTab) -> &'static str {
    match tab {
        T::Profile => "profile",
        T::Security => "security",
        T::Billing => "billing",
        T::Settings => "settings",
        T::Developer => "developer",
    }
}

impl Labeled for TabState {
    fn id(self) -> &'static str {
        match self {
            S::Shows(tab) => tab_id(tab),
            S::NotFound => "not_found",
            S::SignedOut => "signed_out",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Shows(tab) => tab.heading(),
            S::NotFound => "Not found",
            S::SignedOut => "Sign-in page",
        }
    }
}

impl Labeled for TabEvent {
    fn id(self) -> &'static str {
        match self {
            E::Open(T::Profile) => "open_profile",
            E::Open(T::Security) => "open_security",
            E::Open(T::Billing) => "open_billing",
            E::Open(T::Settings) => "open_settings",
            E::Open(T::Developer) => "open_developer",
            E::OpenAccount => "open_account",
            E::OpenUnknown => "open_unknown",
            E::SignOut => "sign_out",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Open(T::Profile) => "Opens the Profile tab's URL",
            E::Open(T::Security) => "Opens the Security tab's URL",
            E::Open(T::Billing) => "Opens the Billing tab's URL",
            E::Open(T::Settings) => "Opens the Settings tab's URL",
            E::Open(T::Developer) => "Opens the Developer tab's URL",
            E::OpenAccount => "Opens /account",
            E::OpenUnknown => "Opens an unknown tab's URL",
            E::SignOut => "Signs out, then reloads",
        }
    }
}

#[cfg(test)]
mod tests;
