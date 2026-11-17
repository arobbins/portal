use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashState {
    Empty,
    PasswordChanged,
    OthersSignedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashEvent {
    ChangePassword,
    SignOutOthers,
    OpenSecurity,
    OpenProfile,
    SignOut,
}

use FlashEvent as E;
use FlashState as S;

pub const FLASH: Machine<FlashState, FlashEvent> = Machine {
    name: "Flash",
    summary: "The one success message waiting in the session after a redirect, until a page that accepts it shows it.",
    states: &[S::Empty, S::PasswordChanged, S::OthersSignedOut],
    terminal: &[],
    events: &[
        E::ChangePassword,
        E::SignOutOthers,
        E::OpenSecurity,
        E::OpenProfile,
        E::SignOut,
    ],
    stored_in: &["session:SESSION_FLASH@definitions"],
    rules: &[SHOWN_EXACTLY_ONCE, OTHER_PAGES_LEAVE_IT, NEVER_IN_THE_URL],
    writers: &[
        Writer {
            caller: "src/flash.rs:set_flash",
            query: "set_flash",
            events: &[E::ChangePassword, E::SignOutOthers],
        },
        Writer {
            caller: "src/flash.rs:take_flash",
            query: "take_flash",
            events: &[E::OpenSecurity],
        },
    ],
};

pub const SHOWN_EXACTLY_ONCE: &str =
    "A flash is shown once, on the first page that accepts it, and never again.";
pub const OTHER_PAGES_LEAVE_IT: &str =
    "A page that doesn't accept the waiting flash neither shows it nor throws it away.";
pub const NEVER_IN_THE_URL: &str =
    "The redirect that sets a flash never puts the message in the URL.";

pub fn expect(from: FlashState, event: FlashEvent) -> Expect<FlashState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (_, E::ChangePassword) => Moves(S::PasswordChanged),
        (_, E::SignOutOthers) => Moves(S::OthersSignedOut),
        (S::Empty, E::OpenSecurity | E::SignOut) | (_, E::OpenProfile) => Refused,
        (_, E::OpenSecurity | E::SignOut) => Moves(S::Empty),
    }
}

impl Labeled for FlashState {
    fn id(self) -> &'static str {
        match self {
            S::Empty => "empty",
            S::PasswordChanged => "password_changed",
            S::OthersSignedOut => "others_signed_out",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Empty => "Nothing waiting",
            S::PasswordChanged => "\"Password changed\" waiting",
            S::OthersSignedOut => "\"Signed out elsewhere\" waiting",
        }
    }
}

impl Labeled for FlashEvent {
    fn id(self) -> &'static str {
        match self {
            E::ChangePassword => "change_password",
            E::SignOutOthers => "sign_out_others",
            E::OpenSecurity => "open_security",
            E::OpenProfile => "open_profile",
            E::SignOut => "sign_out",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::ChangePassword => "Changes the password (no-JS form)",
            E::SignOutOthers => "Signs out other devices",
            E::OpenSecurity => "Opens Security (accepts both)",
            E::OpenProfile => "Opens Profile (accepts neither)",
            E::SignOut => "Signs out",
        }
    }
}

#[cfg(test)]
mod tests;
