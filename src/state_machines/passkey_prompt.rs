use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptState {
    Due,
    Snoozed,
    Declined,
    HasPasskey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptEvent {
    SignIn,
    PickNotNow,
    PickNeverAsk,
    ThirtyDaysPass,
    AddPasskey,
    RemovePasskey,
    ResetPassword,
}

use PromptEvent as E;
use PromptState as S;

pub const PASSKEY_PROMPT: Machine<PromptState, PromptEvent> = Machine {
    name: "Passkey prompt",
    summary: "Whether a password sign-in offers to add a passkey: until the person adds one, says \"Not now\" (30 days) or \"Don't ask again\".",
    states: &[S::Due, S::Snoozed, S::Declined, S::HasPasskey],
    terminal: &[],
    events: &[
        E::SignIn,
        E::PickNotNow,
        E::PickNeverAsk,
        E::ThirtyDaysPass,
        E::AddPasskey,
        E::RemovePasskey,
        E::ResetPassword,
    ],
    stored_in: &["users.passkey_prompt_after", "user_passkeys (row)"],
    rules: &[
        PROMPT_ONLY_WHEN_DUE,
        RESETS_NEVER_PROMPT,
        ANSWERS_ARE_RECORDED,
        PAGE_ONLY_WHEN_DUE,
    ],
    writers: &[
        writer(ANSWER, "query_snooze_passkey_prompt", &[E::PickNotNow]),
        writer(ANSWER, "query_decline_passkey_prompt", &[E::PickNeverAsk]),
        writer(
            "src/passkeys/actions/prompt.rs:never_prompt_for_passkeys",
            "query_decline_passkey_prompt",
            &[],
        ),
        writer(
            "src/passkeys/actions/registration.rs:save_passkey",
            "query_insert_passkey",
            &[E::AddPasskey],
        ),
        writer(
            "src/passkeys/actions/manage.rs:remove_passkey",
            "query_delete_passkey",
            &[E::RemovePasskey],
        ),
    ],
};

const ANSWER: &str = "src/passkeys/actions/prompt.rs:answer_passkey_prompt";

pub const PROMPT_ONLY_WHEN_DUE: &str = "A password sign-in shows the prompt exactly when it's due.";
pub const RESETS_NEVER_PROMPT: &str =
    "Signing in by resetting the password never shows the prompt.";
pub const ANSWERS_ARE_RECORDED: &str =
    "An answer that counts records one account event; an ignored one records none.";
pub const PAGE_ONLY_WHEN_DUE: &str = "The prompt page only opens while the prompt is due.";

const NO_PASSKEY: &str = "There's no passkey to remove.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [PromptEvent],
) -> Writer<PromptEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: PromptState, event: PromptEvent) -> Expect<PromptState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::Due, E::PickNotNow) => Moves(S::Snoozed),
        (S::Due, E::PickNeverAsk) => Moves(S::Declined),
        (S::Snoozed, E::ThirtyDaysPass) => Moves(S::Due),
        (S::HasPasskey, E::RemovePasskey) => Moves(S::Due),
        (S::HasPasskey, E::AddPasskey) => Refused,
        (_, E::AddPasskey) => Moves(S::HasPasskey),
        (_, E::RemovePasskey) => Unreachable(NO_PASSKEY),
        _ => Refused,
    }
}

impl Labeled for PromptState {
    fn id(self) -> &'static str {
        match self {
            S::Due => "due",
            S::Snoozed => "snoozed",
            S::Declined => "declined",
            S::HasPasskey => "has_passkey",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Due => "Due",
            S::Snoozed => "Snoozed for 30 days",
            S::Declined => "Declined for good",
            S::HasPasskey => "Has a passkey",
        }
    }
}

impl Labeled for PromptEvent {
    fn id(self) -> &'static str {
        match self {
            E::SignIn => "sign_in",
            E::PickNotNow => "pick_not_now",
            E::PickNeverAsk => "pick_never_ask",
            E::ThirtyDaysPass => "thirty_days_pass",
            E::AddPasskey => "add_passkey",
            E::RemovePasskey => "remove_passkey",
            E::ResetPassword => "reset_password",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::SignIn => "Signs in with the password",
            E::PickNotNow => "Picks \"Not now\"",
            E::PickNeverAsk => "Picks \"Don't ask again\"",
            E::ThirtyDaysPass => "The snooze runs out",
            E::AddPasskey => "Adds a passkey",
            E::RemovePasskey => "Removes the passkey",
            E::ResetPassword => "Signs in through a reset link",
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits {
    use super::*;
    use crate::passkeys::actions::PROMPT_SNOOZE_DAYS;

    #[test]
    fn the_snooze_in_the_labels_matches_the_limit() {
        let days = format!("{PROMPT_SNOOZE_DAYS} days");
        assert!(PASSKEY_PROMPT.summary.contains(&days));
        assert!(S::Snoozed.label().contains(&days));
    }
}
