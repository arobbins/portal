use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofState {
    None,
    Fresh,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofEvent {
    ConfirmWithPasskey,
    UseIt,
    UseItAndFail,
    WaitSixMinutes,
    ChangePassword,
    RemoveAPasskey,
    SignOutAndIn,
    SignInAsSomeoneElse,
}

use ProofEvent as E;
use ProofState as S;

pub const PASSKEY_REAUTH: Machine<ProofState, ProofEvent> = Machine {
    name: "Passkey reauth grant",
    summary: "The proof a passkey confirmation leaves in the session, which lets the next sensitive action skip the password.",
    states: &[S::None, S::Fresh, S::Stale],
    terminal: &[],
    events: &[
        E::ConfirmWithPasskey,
        E::UseIt,
        E::UseItAndFail,
        E::WaitSixMinutes,
        E::ChangePassword,
        E::RemoveAPasskey,
        E::SignOutAndIn,
        E::SignInAsSomeoneElse,
    ],
    stored_in: &["session:PASSKEY_REAUTH"],
    rules: &[ONLY_FRESH_SKIPS_THE_PASSWORD, ONE_ACTION_PER_PROOF],
    writers: &[
        writer(
            "src/passkeys/session.rs:grant_reauth",
            "grant_passkey_reauth",
            &[E::ConfirmWithPasskey],
        ),
        writer(EMAIL, "take_passkey_proof", &[E::UseIt]),
        writer(EMAIL, "return_passkey_proof", &[E::UseItAndFail]),
        writer(TOKEN, "take_passkey_proof", &[E::UseIt]),
        writer(TOKEN, "return_passkey_proof", &[E::UseItAndFail]),
        writer(CONNECT, "take_passkey_proof", &[E::UseIt]),
        writer(CONNECT, "return_passkey_proof", &[E::UseItAndFail]),
        writer(DELETE, "take_passkey_proof", &[E::UseIt]),
        writer(DELETE, "return_passkey_proof", &[E::UseItAndFail]),
        writer(EXPORT, "take_passkey_proof", &[E::UseIt]),
        writer(EXPORT, "return_passkey_proof", &[E::UseItAndFail]),
        writer(ADMIN, "take_passkey_proof", &[E::UseIt]),
        writer(ADMIN, "return_passkey_proof", &[E::UseItAndFail]),
        writer(ADMIN_DIALOG, "take_passkey_proof", &[E::UseIt]),
        writer(ADMIN_DIALOG, "return_passkey_proof", &[E::UseItAndFail]),
        writer(TWO_FACTOR_TAKE, "take_passkey_proof", &[E::UseIt]),
        writer(
            TWO_FACTOR_RETURN,
            "return_passkey_proof",
            &[E::UseItAndFail],
        ),
        writer(REMOVE, "take_passkey_proof", &[E::RemoveAPasskey]),
        writer(REMOVE, "return_passkey_proof", &[E::UseItAndFail]),
        writer(REMOVE, "forget_passkey_proof", &[E::RemoveAPasskey]),
    ],
};

const EMAIL: &str = "src/profile/pages/email_routes.rs:change_email";
const TOKEN: &str = "src/api_tokens/pages/token_routes.rs:create_token";
const CONNECT: &str = "src/oauth/pages/authorize_page.rs:connect";
const DELETE: &str = "src/profile/pages/delete_routes.rs:remove_account";
const EXPORT: &str = "src/data_export/pages/export_routes.rs:run_export";
const ADMIN: &str = "src/admin/pages/admin_confirm.rs:admin_confirm_submit";
const ADMIN_DIALOG: &str = "src/admin/pages/admin_unlock_routes.rs:unlock_admin_js";
const TWO_FACTOR_TAKE: &str = "src/two_factor/pages/two_factor_routes.rs:proof";
const TWO_FACTOR_RETURN: &str = "src/two_factor/pages/two_factor_routes.rs:failed";
const REMOVE: &str = "src/passkeys/pages/passkey_routes.rs:remove_passkey_submit";

pub const ONLY_FRESH_SKIPS_THE_PASSWORD: &str =
    "A sensitive action goes through without a password only with a fresh proof.";
pub const ONE_ACTION_PER_PROOF: &str =
    "A proof covers one successful action; the next one needs a new confirmation.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [ProofEvent],
) -> Writer<ProofEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: ProofState, event: ProofEvent) -> Expect<ProofState> {
    use Expect::{Moves, Refused};
    match (from, event) {
        (_, E::ConfirmWithPasskey) => Moves(S::Fresh),
        (S::None, _) => Refused,
        (S::Fresh, E::UseItAndFail) => Refused,
        (S::Fresh, E::WaitSixMinutes | E::ChangePassword) => Moves(S::Stale),
        (S::Stale, E::WaitSixMinutes | E::ChangePassword) => Refused,
        (
            _,
            E::UseIt
            | E::UseItAndFail
            | E::RemoveAPasskey
            | E::SignOutAndIn
            | E::SignInAsSomeoneElse,
        ) => Moves(S::None),
    }
}

impl Labeled for ProofState {
    fn id(self) -> &'static str {
        match self {
            S::None => "none",
            S::Fresh => "fresh",
            S::Stale => "stale",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::None => "No proof",
            S::Fresh => "Fresh proof",
            S::Stale => "Stale proof",
        }
    }
}

impl Labeled for ProofEvent {
    fn id(self) -> &'static str {
        match self {
            E::ConfirmWithPasskey => "confirm_with_passkey",
            E::UseIt => "use_it",
            E::UseItAndFail => "use_it_and_fail",
            E::WaitSixMinutes => "wait_six_minutes",
            E::ChangePassword => "change_password",
            E::RemoveAPasskey => "remove_a_passkey",
            E::SignOutAndIn => "sign_out_and_in",
            E::SignInAsSomeoneElse => "sign_in_as_someone_else",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::ConfirmWithPasskey => "Confirms with a passkey",
            E::UseIt => "Changes email without a password",
            E::UseItAndFail => "Tries a change that fails",
            E::WaitSixMinutes => "Waits six minutes",
            E::ChangePassword => "Changes the password",
            E::RemoveAPasskey => "Removes a passkey",
            E::SignOutAndIn => "Signs out and back in",
            E::SignInAsSomeoneElse => "Signs in as someone else here",
        }
    }
}

#[cfg(test)]
mod tests;
