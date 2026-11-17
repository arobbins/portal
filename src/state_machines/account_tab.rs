use super::account_tab_sections::page_id;
use super::{Expect, Labeled, Machine};
use crate::account::tabs::AccountPage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    Shows(AccountPage),
    NotFound,
    SignedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabEvent {
    Open(AccountPage),
    OpenAccount,
    OpenSecurity,
    OpenUnknown,
    SignOut,
}

use AccountPage as P;
use TabEvent as E;
use TabState as S;

pub const ACCOUNT_TAB: Machine<TabState, TabEvent> = Machine {
    name: "Account page",
    summary: "Which account settings page a signed-in browser is looking at, chosen only by the URL.",
    states: &[
        S::Shows(P::Profile),
        S::Shows(P::SignIn),
        S::Shows(P::Devices),
        S::Shows(P::Plan),
        S::Shows(P::Invoices),
        S::Shows(P::Settings),
        S::Shows(P::Developer),
        S::NotFound,
        S::SignedOut,
    ],
    terminal: &[],
    events: &[
        E::Open(P::Profile),
        E::Open(P::SignIn),
        E::Open(P::Devices),
        E::Open(P::Plan),
        E::Open(P::Invoices),
        E::Open(P::Settings),
        E::Open(P::Developer),
        E::OpenAccount,
        E::OpenSecurity,
        E::OpenUnknown,
        E::SignOut,
    ],
    stored_in: &[],
    rules: &[
        ONE_CURRENT_PAGE,
        TITLE_NAMES_THE_PAGE,
        SECTIONS_ARE_ON_THEIR_PAGE,
    ],
    writers: &[],
};

pub const ONE_CURRENT_PAGE: &str =
    "Exactly one page is marked as the current page, and it's the page being shown.";
pub const TITLE_NAMES_THE_PAGE: &str = "The page title names the page being shown.";
pub const SECTIONS_ARE_ON_THEIR_PAGE: &str =
    "Every section the page owns has its anchor on the page, so links to it land.";

const NEEDS_SIGN_IN: &str = "Signing in again isn't part of this machine.";

pub fn expect(from: TabState, event: TabEvent) -> Expect<TabState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::SignedOut, E::SignOut) => Unreachable(NEEDS_SIGN_IN),
        (S::SignedOut, _) => Refused,
        (_, E::Open(page)) => Moves(S::Shows(page)),
        (_, E::OpenAccount) => Moves(S::Shows(P::Profile)),
        (_, E::OpenSecurity) => Moves(S::Shows(P::SignIn)),
        (_, E::OpenUnknown) => Moves(S::NotFound),
        (_, E::SignOut) => Moves(S::SignedOut),
    }
}

impl Labeled for TabState {
    fn id(self) -> &'static str {
        match self {
            S::Shows(page) => page_id(page),
            S::NotFound => "not_found",
            S::SignedOut => "signed_out",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Shows(page) => page.heading(),
            S::NotFound => "Not found",
            S::SignedOut => "Sign-in page",
        }
    }
}

impl Labeled for TabEvent {
    fn id(self) -> &'static str {
        match self {
            E::Open(P::Profile) => "open_profile",
            E::Open(P::SignIn) => "open_sign_in",
            E::Open(P::Devices) => "open_devices",
            E::Open(P::Plan) => "open_plan",
            E::Open(P::Invoices) => "open_invoices",
            E::Open(P::Settings) => "open_settings",
            E::Open(P::Developer) => "open_developer",
            E::OpenAccount => "open_account",
            E::OpenSecurity => "open_security",
            E::OpenUnknown => "open_unknown",
            E::SignOut => "sign_out",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Open(P::Profile) => "Opens the Profile page's URL",
            E::Open(P::SignIn) => "Opens the Sign-in page's URL",
            E::Open(P::Devices) => "Opens the Devices and activity page's URL",
            E::Open(P::Plan) => "Opens the Plan and payment page's URL",
            E::Open(P::Invoices) => "Opens the Invoices page's URL",
            E::Open(P::Settings) => "Opens the Settings page's URL",
            E::Open(P::Developer) => "Opens the Developer page's URL",
            E::OpenAccount => "Opens /account",
            E::OpenSecurity => "Opens /account/security, a section with several pages",
            E::OpenUnknown => "Opens an unknown page's URL",
            E::SignOut => "Signs out, then reloads",
        }
    }
}

#[cfg(test)]
mod tests;
