use super::{Expect, Labeled, Machine};
use crate::account::tabs::{AccountPage, AccountSection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionState {
    Section(AccountSection),
    OnPage(AccountPage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionEvent {
    OpenItsPath,
}

use AccountPage as P;
use AccountSection as A;
use SectionEvent as E;
use SectionState::{OnPage, Section};

pub const ACCOUNT_TAB_SECTIONS: Machine<SectionState, SectionEvent> = Machine {
    name: "Account sections",
    summary: "Where each account settings card lives: its path lands on its page, at its anchor.",
    states: &[
        Section(A::Password),
        Section(A::Passkeys),
        Section(A::TwoFactor),
        Section(A::SingleSignOn),
        Section(A::Devices),
        Section(A::Activity),
        Section(A::YourPlan),
        Section(A::Invoices),
        Section(A::Notifications),
        Section(A::Newsletters),
        Section(A::TimeZone),
        Section(A::ApiTokens),
        Section(A::ExportData),
        Section(A::DeleteAccount),
        OnPage(P::Profile),
        OnPage(P::SignIn),
        OnPage(P::Devices),
        OnPage(P::Plan),
        OnPage(P::Invoices),
        OnPage(P::Notifications),
        OnPage(P::TimeZone),
        OnPage(P::Developer),
    ],
    terminal: &[
        OnPage(P::Profile),
        OnPage(P::SignIn),
        OnPage(P::Devices),
        OnPage(P::Plan),
        OnPage(P::Invoices),
        OnPage(P::Notifications),
        OnPage(P::TimeZone),
        OnPage(P::Developer),
    ],
    events: &[E::OpenItsPath],
    stored_in: &[],
    rules: &[LANDS_AT_ITS_ANCHOR, ONE_HOME_EACH],
    writers: &[],
};

pub const LANDS_AT_ITS_ANCHOR: &str =
    "The page it lands on has the section's anchor, so the browser scrolls to it.";
pub const ONE_HOME_EACH: &str = "Every section belongs to exactly one page.";

const A_PAGE: &str = "A page, not a section.";

pub fn home_of(section: AccountSection) -> AccountPage {
    match section {
        A::Password | A::Passkeys | A::TwoFactor | A::SingleSignOn => P::SignIn,
        A::Devices | A::Activity => P::Devices,
        A::YourPlan => P::Plan,
        A::Invoices => P::Invoices,
        A::Notifications | A::Newsletters => P::Notifications,
        A::TimeZone => P::TimeZone,
        A::ApiTokens => P::Developer,
        A::ExportData | A::DeleteAccount => P::Profile,
    }
}

pub fn expect(from: SectionState, _event: SectionEvent) -> Expect<SectionState> {
    match from {
        Section(section) => Expect::Moves(OnPage(home_of(section))),
        OnPage(_) => Expect::Unreachable(A_PAGE),
    }
}

pub fn page_id(page: AccountPage) -> &'static str {
    match page {
        P::Profile => "profile",
        P::SignIn => "sign_in",
        P::Devices => "devices",
        P::Plan => "plan",
        P::Invoices => "invoices",
        P::Notifications => "notifications",
        P::TimeZone => "time_zone",
        P::Developer => "developer",
    }
}

impl Labeled for SectionState {
    fn id(self) -> &'static str {
        match self {
            Section(section) => section.anchor(),
            OnPage(page) => page_id(page),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Section(section) => section.label(),
            OnPage(page) => page.heading(),
        }
    }
}

impl Labeled for SectionEvent {
    fn id(self) -> &'static str {
        match self {
            E::OpenItsPath => "open_its_path",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::OpenItsPath => "Opens its path directly",
        }
    }
}

#[cfg(test)]
mod tests;
