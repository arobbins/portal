use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use topcoat::{Error, router::StatusCode};
use webauthn_rs::prelude::{Passkey, PublicKeyCredential};

use super::registration::CeremonyProblem;
use crate::action::{ActionProblem, Outcome, ProblemSpec, TooManyAttempts, fail};
use crate::env::Env;
use crate::passkeys::queries::{query_get_passkey_by_credential, query_record_passkey_use};
use crate::passkeys::session::PasskeySession;
use crate::user::model::{AccountStatus, User};
use crate::user::queries::query_get_user_by_webauthn_id;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Start signing in with a passkey.")]
pub struct StartSignIn {
    #[serde(default)]
    #[schemars(description = "Whether the browser offers passkeys as autofill suggestions.")]
    pub conditional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasskeySignInProblem {
    Ceremony(CeremonyProblem),
    UnknownPasskey,
    TooManyAttempts,
}

impl ActionProblem for PasskeySignInProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Ceremony(problem) => problem.spec(),
            Self::UnknownPasskey => ProblemSpec::new(
                "unknown_passkey",
                "That passkey isn't registered with this site anymore. Sign in another way.",
            )
            .with_status(StatusCode::BAD_REQUEST),
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

pub async fn start_passkey_sign_in(
    env: &Env<'_>,
    session: &impl PasskeySession,
    input: &StartSignIn,
) -> Outcome<Value, TooManyAttempts> {
    if !env.limits().allow_passkey_challenge(env.ip).await? {
        return fail(TooManyAttempts);
    }
    let (mut options, state) = env
        .app
        .webauthn
        .start_discoverable_authentication()
        .map_err(|e| Error::msg(e.to_string()))?;
    if !input.conditional {
        options.mediation = None;
    }
    session.store_sign_in(state, env.now()).await?;

    Ok(Ok(
        json!({ "options": options, "origin": env.app.config.origin() }),
    ))
}

pub async fn finish_passkey_sign_in(
    env: &Env<'_>,
    session: &impl PasskeySession,
    credential: &PublicKeyCredential,
) -> Outcome<User, PasskeySignInProblem> {
    if !env.limits().allow_password_submission(env.ip).await? {
        return fail(PasskeySignInProblem::TooManyAttempts);
    }
    let Some(state) = session.take_sign_in(env.now()).await? else {
        return fail(PasskeySignInProblem::Ceremony(CeremonyProblem::Expired));
    };
    let webauthn = &env.app.webauthn;
    let Ok((webauthn_id, credential_id)) =
        webauthn.identify_discoverable_authentication(credential)
    else {
        return fail(PasskeySignInProblem::Ceremony(CeremonyProblem::NotVerified));
    };
    let Some((user, stored)) = registered_passkey(env, webauthn_id, credential_id).await? else {
        return fail(PasskeySignInProblem::UnknownPasskey);
    };

    let mut passkey: Passkey = stored.passkey.0;
    let Ok(result) = webauthn.finish_discoverable_authentication_v2(credential, state, &passkey)
    else {
        return fail(PasskeySignInProblem::Ceremony(CeremonyProblem::NotVerified));
    };
    passkey.update_credential(&result);
    query_record_passkey_use(stored.id, &passkey, result.backup_state(), env.db()).await?;

    Ok(Ok(user))
}

async fn registered_passkey(
    env: &Env<'_>,
    webauthn_id: uuid::Uuid,
    credential_id: &[u8],
) -> Result<Option<(User, crate::passkeys::queries::Passkey)>, sqlx::Error> {
    let user = query_get_user_by_webauthn_id(webauthn_id, env.db())
        .await?
        .filter(|user| matches!(user.status(), AccountStatus::Active { .. }));
    let Some(user) = user else {
        return Ok(None);
    };
    let passkey = query_get_passkey_by_credential(user.id, credential_id, env.db()).await?;

    Ok(passkey.map(|passkey| (user, passkey)))
}
