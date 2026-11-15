use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use topcoat::{Error, router::StatusCode};
use webauthn_rs::prelude::{Credential, CredentialID, Passkey, RegisterPublicKeyCredential};
use webauthn_rs_core::proto::ResidentKeyRequirement;

use crate::account::tabs::AccountSection;
use crate::action::{ActionProblem, ActionResult, Caller, Outcome, ProblemSpec, authorize, fail};
use crate::auth::password::{CurrentPasswordProblem, check_current_password};
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::passkeys::emails::passkey_added_email;
use crate::passkeys::queries::{NewPasskey, query_insert_passkey, query_list_passkeys};
use crate::passkeys::session::PasskeySession;
use crate::passkeys::webauthn::{aaguid_from_attestation, provider_name};
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::User;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeremonyProblem {
    Expired,
    NotVerified,
}

impl ActionProblem for CeremonyProblem {
    fn spec(self) -> ProblemSpec {
        let spec = match self {
            Self::Expired => ProblemSpec::new(
                "passkey_request_expired",
                "That passkey request expired. Please try again.",
            ),
            Self::NotVerified => ProblemSpec::new(
                "passkey_not_verified",
                "We couldn't verify that passkey. Please try again.",
            ),
        };
        spec.with_status(StatusCode::BAD_REQUEST)
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Start adding a passkey.")]
pub struct StartRegistration {
    #[serde(default)]
    #[schemars(
        description = "The current password; may be left out only right after a password sign-in prompt."
    )]
    pub current_password: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartRegistrationProblem {
    CurrentPassword(CurrentPasswordProblem),
    SetupExpired,
}

impl ActionProblem for StartRegistrationProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::CurrentPassword(problem) => problem.spec(),
            Self::SetupExpired => ProblemSpec::new(
                "passkey_setup_expired",
                "This page expired. Add a passkey from your Security settings.",
            )
            .with_status(StatusCode::BAD_REQUEST),
        }
    }
}

pub async fn start_passkey_registration(
    env: &Env<'_>,
    session: &impl PasskeySession,
    caller: &Caller,
    input: &StartRegistration,
) -> ActionResult<Value, StartRegistrationProblem> {
    authorize!(caller.require(Permission::ManagePasskeys));
    if let Err(problem) = confirm_identity(env, session, &caller.user, input).await? {
        return fail(problem);
    }

    let options = begin_ceremony(env, session, &caller.user).await?;
    Ok(Ok(
        json!({ "options": options, "origin": env.app.config.origin() }),
    ))
}

async fn confirm_identity(
    env: &Env<'_>,
    session: &impl PasskeySession,
    user: &User,
    input: &StartRegistration,
) -> Outcome<(), StartRegistrationProblem> {
    if let Some(password) = &input.current_password {
        return Ok(check_current_password(env, user, password)
            .await?
            .map_err(StartRegistrationProblem::CurrentPassword));
    }
    let limits = env.limits();
    if !limits.allow_password_submission(env.ip).await?
        || limits.login_blocked(&user.email, env.ip).await?
    {
        return fail(StartRegistrationProblem::CurrentPassword(
            CurrentPasswordProblem::TooManyAttempts,
        ));
    }
    if !session.prompt_granted(user, env.now()).await? {
        return fail(StartRegistrationProblem::SetupExpired);
    }
    Ok(Ok(()))
}

async fn begin_ceremony(
    env: &Env<'_>,
    session: &impl PasskeySession,
    user: &User,
) -> topcoat::Result<Value> {
    let existing: Vec<CredentialID> = query_list_passkeys(user.id, env.db())
        .await?
        .iter()
        .map(|passkey| passkey.passkey.cred_id().clone())
        .collect();
    let (mut options, state) = env
        .app
        .webauthn
        .start_passkey_registration(user.webauthn_id, &user.email, &user.name, Some(existing))
        .map_err(|e| Error::msg(e.to_string()))?;
    if let Some(selection) = options.public_key.authenticator_selection.as_mut() {
        selection.resident_key = Some(ResidentKeyRequirement::Required);
        selection.require_resident_key = true;
    }
    session.store_registration(state, env.now()).await?;

    serde_json::to_value(options).map_err(|error| Error::msg(error.to_string()))
}

pub async fn finish_passkey_registration(
    env: &Env<'_>,
    session: &impl PasskeySession,
    caller: &Caller,
    credential: &RegisterPublicKeyCredential,
) -> ActionResult<(), CeremonyProblem> {
    authorize!(caller.require(Permission::ManagePasskeys));
    let Some(state) = session.take_registration(env.now()).await? else {
        return fail(CeremonyProblem::Expired);
    };
    let Ok(passkey) = env
        .app
        .webauthn
        .finish_passkey_registration(credential, &state)
    else {
        return fail(CeremonyProblem::NotVerified);
    };

    let aaguid = aaguid_from_attestation(credential.response.attestation_object.as_ref());
    save_passkey(env, caller, &passkey, provider_name(aaguid), aaguid).await?;
    session.revoke_prompt().await?;

    Ok(Ok(()))
}

async fn save_passkey(
    env: &Env<'_>,
    caller: &Caller,
    passkey: &Passkey,
    name: &str,
    aaguid: Option<uuid::Uuid>,
) -> topcoat::Result<()> {
    let details = Credential::from(passkey.clone());
    let public_id = PublicIdKind::Passkey.generate();
    let new_passkey = NewPasskey {
        user_id: caller.user.id,
        public_id: &public_id,
        passkey,
        name,
        aaguid,
        backup_eligible: details.backup_eligible,
        backed_up: details.backup_state,
    };
    let mut tx = env.db().begin().await?;
    query_insert_passkey(&new_passkey, &mut *tx).await?;
    let detail = json!({ "passkey": public_id, "name": name });
    record_event(
        env,
        &EventSource::of(caller),
        AccountEventKind::PasskeyAdded,
        detail,
        &mut *tx,
    )
    .await?;
    let security_url = env.app.url(&AccountSection::Passkeys.path());
    let email = passkey_added_email(&caller.user.email, name, &security_url);
    enqueue(&SendEmail(email), &mut *tx).await?;
    tx.commit().await?;

    Ok(())
}
