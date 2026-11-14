use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use topcoat::{
    Error,
    context::Cx,
    router::{StatusCode, request::client_ip},
};
use webauthn_rs::prelude::{
    Credential, CredentialID, DiscoverableAuthentication, Passkey, PasskeyRegistration,
    PublicKeyCredential, RegisterPublicKeyCredential,
};
use webauthn_rs_core::proto::ResidentKeyRequirement;

use crate::account::tabs::AccountSection;
use crate::action::{
    ActionProblem, ActionResult, Caller, Outcome, TooManyAttempts, authorize, fail,
};
use crate::auth::password::{CurrentPasswordProblem, check_current_password};
use crate::auth::session::{passkey_prompt_granted, revoke_passkey_prompt};
use crate::jobs::{Job, enqueue};
use crate::passkeys::emails::passkey_added_email;
use crate::passkeys::queries::{
    NewPasskey, Passkey as StoredPasskey, query_decline_passkey_prompt, query_delete_passkey,
    query_get_passkey_by_credential, query_insert_passkey, query_list_passkeys,
    query_record_passkey_use, query_rename_passkey, query_snooze_passkey_prompt,
};
use crate::passkeys::webauthn::{
    AUTHENTICATION_STATE, REGISTRATION_STATE, aaguid_from_attestation, provider_name,
    store_ceremony, take_ceremony,
};
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::{AccountStatus, User};
use crate::user::name::{MAX_NAME_LENGTH, NameProblem, check_name};
use crate::user::queries::query_get_user_by_webauthn_id;
use crate::web::{app_url, config, db, rate_limits, session, webauthn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeremonyProblem {
    Expired,
    NotVerified,
}

impl ActionProblem for CeremonyProblem {
    fn code(self) -> &'static str {
        match self {
            Self::Expired => "passkey_request_expired",
            Self::NotVerified => "passkey_not_verified",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Expired => "That passkey request expired. Please try again.",
            Self::NotVerified => "We couldn't verify that passkey. Please try again.",
        }
    }

    fn status(self) -> StatusCode {
        StatusCode::BAD_REQUEST
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
    fn code(self) -> &'static str {
        match self {
            Self::CurrentPassword(problem) => problem.code(),
            Self::SetupExpired => "passkey_setup_expired",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::CurrentPassword(problem) => problem.message(),
            Self::SetupExpired => "This page expired. Add a passkey from your Security settings.",
        }
    }

    fn field(self) -> Option<&'static str> {
        match self {
            Self::CurrentPassword(problem) => problem.field(),
            Self::SetupExpired => None,
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::CurrentPassword(problem) => problem.status(),
            Self::SetupExpired => StatusCode::BAD_REQUEST,
        }
    }
}

pub async fn start_passkey_registration(
    cx: &Cx,
    caller: &Caller,
    input: &StartRegistration,
) -> ActionResult<Value, StartRegistrationProblem> {
    authorize!(caller.require(Permission::ManagePasskeys));
    let user = &caller.user;
    let session = session(cx)?;
    match &input.current_password {
        Some(password) => {
            if let Err(problem) = check_current_password(cx, user, password).await? {
                return fail(StartRegistrationProblem::CurrentPassword(problem));
            }
        }
        None => {
            let limits = rate_limits(cx);
            let ip = client_ip(cx);
            if !limits.allow_password_submission(ip).await?
                || limits.login_blocked(&user.email, ip).await?
            {
                return fail(StartRegistrationProblem::CurrentPassword(
                    CurrentPasswordProblem::TooManyAttempts,
                ));
            }
            if !passkey_prompt_granted(&session, user).await? {
                return fail(StartRegistrationProblem::SetupExpired);
            }
        }
    }

    let existing: Vec<CredentialID> = query_list_passkeys(user.id, db(cx))
        .await?
        .iter()
        .map(|passkey| passkey.passkey.cred_id().clone())
        .collect();
    let (mut options, state) = webauthn(cx)
        .start_passkey_registration(user.webauthn_id, &user.email, &user.name, Some(existing))
        .map_err(|e| Error::msg(e.to_string()))?;
    if let Some(selection) = options.public_key.authenticator_selection.as_mut() {
        selection.resident_key = Some(ResidentKeyRequirement::Required);
        selection.require_resident_key = true;
    }
    store_ceremony::<PasskeyRegistration>(&session, REGISTRATION_STATE, state).await?;

    Ok(Ok(
        json!({ "options": options, "origin": config(cx).origin() }),
    ))
}

pub async fn finish_passkey_registration(
    cx: &Cx,
    caller: &Caller,
    credential: &RegisterPublicKeyCredential,
) -> ActionResult<(), CeremonyProblem> {
    authorize!(caller.require(Permission::ManagePasskeys));
    let user = &caller.user;
    let session = session(cx)?;
    let Some(state) = take_ceremony::<PasskeyRegistration>(&session, REGISTRATION_STATE).await?
    else {
        return fail(CeremonyProblem::Expired);
    };
    let Ok(passkey) = webauthn(cx).finish_passkey_registration(credential, &state) else {
        return fail(CeremonyProblem::NotVerified);
    };

    let details = Credential::from(passkey.clone());
    let aaguid = aaguid_from_attestation(credential.response.attestation_object.as_ref());
    let name = provider_name(aaguid);
    let public_id = PublicIdKind::Passkey.generate();
    let mut tx = db(cx).begin().await?;
    query_insert_passkey(
        &NewPasskey {
            user_id: user.id,
            public_id: &public_id,
            passkey: &passkey,
            name,
            aaguid,
            backup_eligible: details.backup_eligible,
            backed_up: details.backup_state,
        },
        &mut *tx,
    )
    .await?;
    record_event(
        cx,
        &EventSource::of(caller),
        AccountEventKind::PasskeyAdded,
        json!({ "passkey": public_id, "name": name }),
        &mut *tx,
    )
    .await?;
    let security_url = app_url(cx, &AccountSection::Passkeys.href(cx));
    let email = passkey_added_email(&user.email, name, &security_url);
    enqueue(&Job::SendEmail(email), &mut *tx).await?;
    tx.commit().await?;
    revoke_passkey_prompt(&session).await?;

    Ok(Ok(()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasskeyNotFound {
    NotFound,
}

impl ActionProblem for PasskeyNotFound {
    fn code(self) -> &'static str {
        "passkey_not_found"
    }

    fn message(self) -> &'static str {
        "That passkey doesn't exist."
    }

    fn status(self) -> StatusCode {
        StatusCode::NOT_FOUND
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Rename a passkey.")]
pub struct RenamePasskey {
    #[schemars(description = "The passkey's id (pk_…).")]
    pub id: String,
    #[schemars(description = "The new name.", length(min = 1, max = MAX_NAME_LENGTH))]
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenamePasskeyProblem {
    Name(NameProblem),
    NotFound,
}

impl ActionProblem for RenamePasskeyProblem {
    fn code(self) -> &'static str {
        match self {
            Self::Name(problem) => problem.code(),
            Self::NotFound => PasskeyNotFound::NotFound.code(),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Name(problem) => problem.message(),
            Self::NotFound => PasskeyNotFound::NotFound.message(),
        }
    }

    fn field(self) -> Option<&'static str> {
        match self {
            Self::Name(problem) => problem.field(),
            Self::NotFound => None,
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::Name(problem) => problem.status(),
            Self::NotFound => PasskeyNotFound::NotFound.status(),
        }
    }
}

pub async fn rename_passkey(
    cx: &Cx,
    caller: &Caller,
    input: &RenamePasskey,
) -> ActionResult<(), RenamePasskeyProblem> {
    authorize!(caller.require(Permission::ManagePasskeys));
    let name = match check_name(&input.name) {
        Ok(name) => name,
        Err(problem) => return fail(RenamePasskeyProblem::Name(problem)),
    };
    if !PublicIdKind::Passkey.is_valid(&input.id) {
        return fail(RenamePasskeyProblem::NotFound);
    }

    let mut tx = db(cx).begin().await?;
    if !query_rename_passkey(&input.id, caller.owner_id(), name, &mut *tx).await? {
        return fail(RenamePasskeyProblem::NotFound);
    }
    record_event(
        cx,
        &EventSource::of(caller),
        AccountEventKind::PasskeyRenamed,
        json!({ "passkey": input.id, "name": name }),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(()))
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Remove a passkey.")]
pub struct RemovePasskey {
    #[schemars(description = "The passkey's id (pk_…).")]
    pub id: String,
}

pub async fn remove_passkey(
    cx: &Cx,
    caller: &Caller,
    input: &RemovePasskey,
) -> ActionResult<String, PasskeyNotFound> {
    authorize!(caller.require(Permission::ManagePasskeys));
    if !PublicIdKind::Passkey.is_valid(&input.id) {
        return fail(PasskeyNotFound::NotFound);
    }

    let mut tx = db(cx).begin().await?;
    let Some(name) = query_delete_passkey(&input.id, caller.owner_id(), &mut *tx).await? else {
        return fail(PasskeyNotFound::NotFound);
    };
    record_event(
        cx,
        &EventSource::of(caller),
        AccountEventKind::PasskeyRemoved,
        json!({ "passkey": input.id, "name": name }),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(name))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptAnswer {
    NotNow,
    Never,
}

pub const PROMPT_SNOOZE_DAYS: i32 = 30;

pub async fn answer_passkey_prompt(
    cx: &Cx,
    caller: &Caller,
    answer: PromptAnswer,
) -> ActionResult<(), Infallible> {
    authorize!(caller.require(Permission::ManagePasskeys));
    let mut tx = db(cx).begin().await?;
    let kind = match answer {
        PromptAnswer::NotNow => {
            query_snooze_passkey_prompt(caller.user.id, PROMPT_SNOOZE_DAYS, &mut *tx).await?;
            AccountEventKind::PasskeyPromptSnoozed
        }
        PromptAnswer::Never => {
            query_decline_passkey_prompt(caller.user.id, &mut *tx).await?;
            AccountEventKind::PasskeyPromptDeclined
        }
    };
    record_event(cx, &EventSource::of(caller), kind, json!({}), &mut *tx).await?;
    tx.commit().await?;
    revoke_passkey_prompt(&session(cx)?).await?;

    Ok(Ok(()))
}

pub async fn list_passkeys(
    cx: &Cx,
    caller: &Caller,
) -> ActionResult<Vec<StoredPasskey>, Infallible> {
    authorize!(caller.require(Permission::ManagePasskeys));

    Ok(Ok(query_list_passkeys(caller.owner_id(), db(cx)).await?))
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Start signing in with a passkey.")]
pub struct StartSignIn {
    #[serde(default)]
    #[schemars(description = "Whether the browser offers passkeys as autofill suggestions.")]
    pub conditional: bool,
}

pub async fn start_passkey_sign_in(
    cx: &Cx,
    input: &StartSignIn,
) -> Outcome<Value, TooManyAttempts> {
    if !rate_limits(cx)
        .allow_passkey_challenge(client_ip(cx))
        .await?
    {
        return fail(TooManyAttempts);
    }
    let (mut options, state) = webauthn(cx)
        .start_discoverable_authentication()
        .map_err(|e| Error::msg(e.to_string()))?;
    if !input.conditional {
        options.mediation = None;
    }
    store_ceremony::<DiscoverableAuthentication>(&session(cx)?, AUTHENTICATION_STATE, state)
        .await?;

    Ok(Ok(
        json!({ "options": options, "origin": config(cx).origin() }),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasskeySignInProblem {
    Ceremony(CeremonyProblem),
    UnknownPasskey,
    TooManyAttempts,
}

impl ActionProblem for PasskeySignInProblem {
    fn code(self) -> &'static str {
        match self {
            Self::Ceremony(problem) => problem.code(),
            Self::UnknownPasskey => "unknown_passkey",
            Self::TooManyAttempts => TooManyAttempts.code(),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Ceremony(problem) => problem.message(),
            Self::UnknownPasskey => {
                "That passkey isn't registered with this site anymore. Sign in another way."
            }
            Self::TooManyAttempts => TooManyAttempts.message(),
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::Ceremony(problem) => problem.status(),
            Self::UnknownPasskey => StatusCode::BAD_REQUEST,
            Self::TooManyAttempts => TooManyAttempts.status(),
        }
    }
}

pub async fn finish_passkey_sign_in(
    cx: &Cx,
    credential: &PublicKeyCredential,
) -> Outcome<User, PasskeySignInProblem> {
    if !rate_limits(cx)
        .allow_password_submission(client_ip(cx))
        .await?
    {
        return fail(PasskeySignInProblem::TooManyAttempts);
    }
    let session = session(cx)?;
    let Some(state) =
        take_ceremony::<DiscoverableAuthentication>(&session, AUTHENTICATION_STATE).await?
    else {
        return fail(PasskeySignInProblem::Ceremony(CeremonyProblem::Expired));
    };
    let Ok((webauthn_id, credential_id)) =
        webauthn(cx).identify_discoverable_authentication(credential)
    else {
        return fail(PasskeySignInProblem::Ceremony(CeremonyProblem::NotVerified));
    };

    let user = query_get_user_by_webauthn_id(webauthn_id, db(cx))
        .await?
        .filter(|user| matches!(user.status(), AccountStatus::Active { .. }));
    let Some(user) = user else {
        return fail(PasskeySignInProblem::UnknownPasskey);
    };
    let Some(stored) = query_get_passkey_by_credential(user.id, credential_id, db(cx)).await?
    else {
        return fail(PasskeySignInProblem::UnknownPasskey);
    };
    let mut passkey: Passkey = stored.passkey.0;
    let Ok(result) =
        webauthn(cx).finish_discoverable_authentication_v2(credential, state, &passkey)
    else {
        return fail(PasskeySignInProblem::Ceremony(CeremonyProblem::NotVerified));
    };
    passkey.update_credential(&result);
    query_record_passkey_use(stored.id, &passkey, result.backup_state(), db(cx)).await?;

    Ok(Ok(user))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, Denied};
    use crate::api::scope::ApiScope;
    use sqlx::PgPool;
    use std::sync::Arc;

    #[sqlx::test]
    async fn an_api_token_can_never_change_passkeys(db: PgPool) {
        let cx = Cx::new().with(db.clone());
        let caller = Caller::api_token(
            Arc::new(User::default_for_tests()),
            1,
            ApiScope::ALL.to_vec(),
        );
        let id = PublicIdKind::Passkey.generate();

        let renamed = rename_passkey(
            &cx,
            &caller,
            &RenamePasskey {
                id: id.clone(),
                name: "Sneaky".to_string(),
            },
        )
        .await
        .unwrap();
        let removed = remove_passkey(&cx, &caller, &RemovePasskey { id })
            .await
            .unwrap();

        assert_eq!(renamed, Err(ActionError::Denied(Denied::BrowserOnly)));
        assert_eq!(removed, Err(ActionError::Denied(Denied::BrowserOnly)));
    }
}
