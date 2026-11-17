use super::{Expect, Labeled, Machine};
use crate::account::tabs::{AccountSection, AccountTab};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionState {
    Section(AccountSection),
    OnTab(AccountTab),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionEvent {
    FollowSidebarLink,
    OpenItsPath,
}

use AccountSection as A;
use AccountTab as T;
use SectionEvent as E;
use SectionState::{OnTab, Section};

pub const ACCOUNT_TAB_SECTIONS: Machine<SectionState, SectionEvent> = Machine {
    name: "Account tab sections",
    summary: "Where each account settings section lives: its sidebar link and its path both land on its tab, at its anchor.",
    states: &[
        Section(A::Password),
        Section(A::Passkeys),
        Section(A::TwoFactor),
        Section(A::SingleSignOn),
        Section(A::Devices),
        Section(A::Activity),
        Section(A::Subscriptions),
        Section(A::Invoices),
        Section(A::PaymentMethods),
        Section(A::Notifications),
        Section(A::ApiTokens),
        Section(A::DeleteAccount),
        OnTab(T::Profile),
        OnTab(T::Security),
        OnTab(T::Billing),
        OnTab(T::Settings),
        OnTab(T::Developer),
    ],
    terminal: &[
        OnTab(T::Profile),
        OnTab(T::Security),
        OnTab(T::Billing),
        OnTab(T::Settings),
        OnTab(T::Developer),
    ],
    events: &[E::FollowSidebarLink, E::OpenItsPath],
    stored_in: &[],
    rules: &[LANDS_AT_ITS_ANCHOR, ONE_HOME_EACH],
    writers: &[],
};

pub const LANDS_AT_ITS_ANCHOR: &str =
    "The page it lands on has the section's anchor, so the browser scrolls to it.";
pub const ONE_HOME_EACH: &str = "Every section belongs to exactly one tab.";

const A_TAB: &str = "A tab, not a section.";

pub fn home_of(section: AccountSection) -> AccountTab {
    match section {
        A::Password | A::Passkeys | A::TwoFactor | A::SingleSignOn | A::Devices | A::Activity => {
            T::Security
        }
        A::Subscriptions | A::Invoices | A::PaymentMethods => T::Billing,
        A::Notifications => T::Settings,
        A::ApiTokens => T::Developer,
        A::DeleteAccount => T::Profile,
    }
}

pub fn expect(from: SectionState, _event: SectionEvent) -> Expect<SectionState> {
    match from {
        Section(section) => Expect::Moves(OnTab(home_of(section))),
        OnTab(_) => Expect::Unreachable(A_TAB),
    }
}

impl Labeled for SectionState {
    fn id(self) -> &'static str {
        match self {
            Section(section) => section.anchor(),
            OnTab(tab) => tab.slug(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Section(section) => section.label(),
            OnTab(tab) => tab.heading(),
        }
    }
}

impl Labeled for SectionEvent {
    fn id(self) -> &'static str {
        match self {
            E::FollowSidebarLink => "follow_sidebar_link",
            E::OpenItsPath => "open_its_path",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::FollowSidebarLink => "Follows its sidebar link",
            E::OpenItsPath => "Opens its path directly",
        }
    }
}

#[cfg(test)]
mod tests;
