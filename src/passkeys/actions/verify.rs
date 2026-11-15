use serde_json::{Value, json};
use topcoat::{Error, router::StatusCode};
use webauthn_rs::prelude::{Passkey, PublicKeyCredential};

use super::registration::CeremonyProblem;
use crate::action::{
    ActionProblem, ActionResult, Caller, ProblemSpec, TooManyAttempts, authorize, fail,
};
use crate::env::Env;
use crate::passkeys::queries::{
    query_get_passkey_by_credential, query_list_passkeys, query_record_passkey_use,
    query_user_has_passkey,
};
use crate::passkeys::session::VerificationSession;
use crate::permission::Permission;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyPasskeyProblem {
    NoPasskeys,
    Ceremony(CeremonyProblem),
    TooManyAttempts,
}

impl ActionProblem for VerifyPasskeyProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::NoPasskeys => ProblemSpec::new(
                "no_passkeys",
                "You haven't added a passkey yet. Enter your current password instead.",
            )
            .with_status(StatusCode::BAD_REQUEST),
            Self::Ceremony(problem) => problem.spec(),
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

pub async fn has_passkey(env: &Env<'_>, user_id: i32) -> Result<bool, sqlx::Error> {
    query_user_has_passkey(user_id, env.db()).await
}

pub async fn start_passkey_verification(
    env: &Env<'_>,
    session: &impl VerificationSession,
    caller: &Caller,
) -> ActionResult<Value, VerifyPasskeyProblem> {
    authorize!(caller.require(Permission::ConfirmIdentity));
    if !env.limits().allow_passkey_challenge(env.ip).await? {
        return fail(VerifyPasskeyProblem::TooManyAttempts);
    }
    let passkeys: Vec<Passkey> = query_list_passkeys(caller.user.id, env.db())
        .await?
        .into_iter()
        .map(|stored| stored.passkey.0)
        .collect();
    if passkeys.is_empty() {
        return fail(VerifyPasskeyProblem::NoPasskeys);
    }
    let (options, state) = env
        .app
        .webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|e| Error::msg(e.to_string()))?;
    session.store_verification(state, env.now()).await?;

    Ok(Ok(
        json!({ "options": options, "origin": env.app.config.origin() }),
    ))
}

pub async fn finish_passkey_verification(
    env: &Env<'_>,
    session: &impl VerificationSession,
    caller: &Caller,
    credential: &PublicKeyCredential,
) -> ActionResult<(), VerifyPasskeyProblem> {
    authorize!(caller.require(Permission::ConfirmIdentity));
    let limits = env.limits();
    let user = &caller.user;
    if !limits.allow_password_submission(env.ip).await?
        || limits.login_blocked(&user.email, env.ip).await?
    {
        return fail(VerifyPasskeyProblem::TooManyAttempts);
    }
    let Some(state) = session.take_verification(env.now()).await? else {
        return fail(VerifyPasskeyProblem::Ceremony(CeremonyProblem::Expired));
    };
    let not_verified = VerifyPasskeyProblem::Ceremony(CeremonyProblem::NotVerified);
    let webauthn = &env.app.webauthn;
    let Ok(result) = webauthn.finish_passkey_authentication(credential, &state) else {
        return fail(not_verified);
    };
    let credential_id: &[u8] = result.cred_id().as_ref();
    let Some(stored) = query_get_passkey_by_credential(user.id, credential_id, env.db()).await?
    else {
        return fail(not_verified);
    };

    let mut passkey: Passkey = stored.passkey.0;
    passkey.update_credential(&result);
    query_record_passkey_use(stored.id, &passkey, result.backup_state(), env.db()).await?;
    session.grant_reauth(user, env.now()).await?;
    Ok(Ok(()))
}

#[cfg(test)]
mod tests;
