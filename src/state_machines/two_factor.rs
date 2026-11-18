use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoFactorState {
    Off,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoFactorEvent {
    TurnOn,
    TurnOnWrongCode,
    TurnOnWrongPassword,
    NewRecoveryCodes,
    TurnOff,
    TurnOffWrongPassword,
}

use TwoFactorEvent as E;
use TwoFactorState as S;

const ENABLE: &str = "src/two_factor/actions.rs:enable_two_factor";
const DISABLE: &str = "src/two_factor/actions.rs:disable_two_factor";
const REPLACE: &str = "src/two_factor/actions.rs:replace_recovery_codes";
const APP_CODE: &str = "src/two_factor/actions/sign_in.rs:accepts_app_code";
const RECOVERY_CODE: &str = "src/two_factor/actions/sign_in.rs:accepts_recovery_code";

pub const TWO_FACTOR: Machine<TwoFactorState, TwoFactorEvent> = Machine {
    name: "Two-factor",
    summary: "Whether signing in with a password also needs a code from an authenticator app, and the recovery codes that stand in for it.",
    states: &[S::Off, S::On],
    terminal: &[],
    events: &[
        E::TurnOn,
        E::TurnOnWrongCode,
        E::TurnOnWrongPassword,
        E::NewRecoveryCodes,
        E::TurnOff,
        E::TurnOffWrongPassword,
    ],
    stored_in: &[
        "user_two_factor",
        "user_two_factor.*",
        "recovery_codes",
        "recovery_codes.*",
    ],
    rules: &[CHANGES_ARE_EMAILED, CODES_SHOWN_ONCE, OFF_KEEPS_NOTHING],
    writers: &[
        Writer {
            caller: ENABLE,
            query: "query_insert_two_factor",
            events: &[E::TurnOn],
        },
        Writer {
            caller: DISABLE,
            query: "query_delete_two_factor",
            events: &[E::TurnOff],
        },
        Writer {
            caller: DISABLE,
            query: "query_delete_recovery_codes",
            events: &[E::TurnOff],
        },
        Writer {
            caller: REPLACE,
            query: "query_delete_recovery_codes",
            events: &[E::TurnOn, E::NewRecoveryCodes],
        },
        Writer {
            caller: REPLACE,
            query: "query_insert_recovery_codes",
            events: &[E::TurnOn, E::NewRecoveryCodes],
        },
        Writer {
            caller: APP_CODE,
            query: "query_use_step",
            events: &[],
        },
        Writer {
            caller: RECOVERY_CODE,
            query: "query_use_recovery_code",
            events: &[],
        },
    ],
};

pub const CHANGES_ARE_EMAILED: &str =
    "Turning two-factor on or off always queues an email to the account.";
pub const CODES_SHOWN_ONCE: &str = "Turning on or asking for new codes shows exactly ten recovery codes, stored only as hashes, and they aren't shown again on the next visit.";
pub const OFF_KEEPS_NOTHING: &str = "While off, no secret and no recovery codes are kept.";

pub fn expect(from: TwoFactorState, event: TwoFactorEvent) -> Expect<TwoFactorState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (S::Off, E::TurnOn) => Moves(S::On),
        (S::Off, _) => Refused,
        (S::On, E::NewRecoveryCodes) => Moves(S::On),
        (S::On, E::TurnOff) => Moves(S::Off),
        (S::On, _) => Refused,
    }
}

impl Labeled for TwoFactorState {
    fn id(self) -> &'static str {
        match self {
            S::Off => "off",
            S::On => "on",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Off => "Off",
            S::On => "On",
        }
    }
}

impl Labeled for TwoFactorEvent {
    fn id(self) -> &'static str {
        match self {
            E::TurnOn => "turn_on",
            E::TurnOnWrongCode => "turn_on_wrong_code",
            E::TurnOnWrongPassword => "turn_on_wrong_password",
            E::NewRecoveryCodes => "new_recovery_codes",
            E::TurnOff => "turn_off",
            E::TurnOffWrongPassword => "turn_off_wrong_password",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::TurnOn => "Turns on with the password and a code from the app",
            E::TurnOnWrongCode => "Turns on with a wrong code",
            E::TurnOnWrongPassword => "Turns on with a wrong password",
            E::NewRecoveryCodes => "Asks for new recovery codes",
            E::TurnOff => "Turns off with the password",
            E::TurnOffWrongPassword => "Turns off with a wrong password",
        }
    }
}

#[cfg(test)]
mod tests;
