use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgConnection;
use time::OffsetDateTime;

use super::codes::{is_secret, matching_step, new_recovery_codes, normalize_recovery_code};
use super::emails::{two_factor_disabled_email, two_factor_enabled_email};
use super::queries::{
    query_count_recovery_codes_left, query_delete_recovery_codes, query_delete_two_factor,
    query_get_two_factor, query_insert_recovery_codes, query_insert_two_factor,
};
use crate::account::tabs::AccountSection;
use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::auth::reauth::{PasskeyProof, ReauthProblem, check_reauth};
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::{Email, SendEmail};
use crate::permission::Permission;
use crate::secret::hash_secret;
use crate::user::events::{AccountEventKind, EventSource, record_event};

mod sign_in;

pub use sign_in::{
    SecondFactorProblem, needs_second_factor, second_factor_matches, verify_second_factor,
};

pub const CODE_FIELD: &str = "code";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoFactorSummary {
    pub enabled_at: Option<OffsetDateTime>,
    pub recovery_codes_left: i64,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Turn on two-factor authentication with an authenticator app.")]
pub struct EnableTwoFactor {
    #[schemars(
        description = "The setup key shown with the QR code.",
        length(equal = 32)
    )]
    pub secret: String,
    #[schemars(
        description = "A 6-digit code from the authenticator app.",
        length(max = 16)
    )]
    pub code: String,
    #[serde(default)]
    #[schemars(description = "Your current password, unless you confirmed with a passkey.")]
    pub current_password: String,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Confirm it's you before changing two-factor authentication.")]
pub struct ConfirmTwoFactorChange {
    #[serde(default)]
    #[schemars(description = "Your current password, unless you confirmed with a passkey.")]
    pub current_password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoFactorProblem {
    Reauth(ReauthProblem),
    WrongCode,
    AlreadyOn,
    NotOn,
}

impl ActionProblem for TwoFactorProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Reauth(problem) => problem.spec(),
            Self::WrongCode => ProblemSpec::new(
                "wrong_code",
                "That code didn't match. Check that your phone's clock is right and enter the newest code.",
            )
            .on_field(CODE_FIELD),
            Self::AlreadyOn => ProblemSpec::new(
                "two_factor_on",
                "Two-factor authentication is already on.",
            ),
            Self::NotOn => ProblemSpec::new(
                "two_factor_off",
                "Two-factor authentication is off.",
            ),
        }
    }
}

pub async fn get_two_factor(
    env: &Env<'_>,
    caller: &Caller,
) -> ActionResult<TwoFactorSummary, Infallible> {
    authorize!(caller.require(Permission::ManageTwoFactor));
    let row = query_get_two_factor(caller.user.id, env.db()).await?;
    let recovery_codes_left = query_count_recovery_codes_left(caller.user.id, env.db()).await?;

    Ok(Ok(TwoFactorSummary {
        enabled_at: row.map(|row| row.enabled_at),
        recovery_codes_left,
    }))
}

pub async fn enable_two_factor(
    env: &Env<'_>,
    caller: &Caller,
    input: &EnableTwoFactor,
    passkey: Option<PasskeyProof>,
) -> ActionResult<Vec<String>, TwoFactorProblem> {
    authorize!(caller.require(Permission::ManageTwoFactor));
    if let Err(problem) = check_reauth(env, &caller.user, &input.current_password, passkey).await? {
        return fail(TwoFactorProblem::Reauth(problem));
    }
    let step = is_secret(&input.secret)
        .then(|| matching_step(&input.secret, &input.code, env.now().unix_timestamp()))
        .flatten();
    let Some(step) = step else {
        return fail(TwoFactorProblem::WrongCode);
    };

    let mut tx = env.db().begin().await?;
    if !query_insert_two_factor(caller.user.id, &input.secret, step, &mut *tx).await? {
        return fail(TwoFactorProblem::AlreadyOn);
    }
    let codes = replace_recovery_codes(caller.user.id, &mut tx).await?;
    log_and_tell(
        env,
        caller,
        AccountEventKind::TwoFactorEnabled,
        two_factor_enabled_email,
        &mut tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(codes))
}

pub async fn disable_two_factor(
    env: &Env<'_>,
    caller: &Caller,
    input: &ConfirmTwoFactorChange,
    passkey: Option<PasskeyProof>,
) -> ActionResult<(), TwoFactorProblem> {
    authorize!(caller.require(Permission::ManageTwoFactor));
    if let Err(problem) = check_reauth(env, &caller.user, &input.current_password, passkey).await? {
        return fail(TwoFactorProblem::Reauth(problem));
    }

    let mut tx = env.db().begin().await?;
    if !query_delete_two_factor(caller.user.id, &mut *tx).await? {
        return fail(TwoFactorProblem::NotOn);
    }
    query_delete_recovery_codes(caller.user.id, &mut *tx).await?;
    log_and_tell(
        env,
        caller,
        AccountEventKind::TwoFactorDisabled,
        two_factor_disabled_email,
        &mut tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(()))
}

pub async fn regenerate_recovery_codes(
    env: &Env<'_>,
    caller: &Caller,
    input: &ConfirmTwoFactorChange,
    passkey: Option<PasskeyProof>,
) -> ActionResult<Vec<String>, TwoFactorProblem> {
    authorize!(caller.require(Permission::ManageTwoFactor));
    if let Err(problem) = check_reauth(env, &caller.user, &input.current_password, passkey).await? {
        return fail(TwoFactorProblem::Reauth(problem));
    }

    let mut tx = env.db().begin().await?;
    if query_get_two_factor(caller.user.id, &mut *tx)
        .await?
        .is_none()
    {
        return fail(TwoFactorProblem::NotOn);
    }
    let codes = replace_recovery_codes(caller.user.id, &mut tx).await?;
    let source = EventSource::of(caller);
    record_event(
        env,
        &source,
        AccountEventKind::RecoveryCodesRegenerated,
        json!({}),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(codes))
}

async fn replace_recovery_codes(
    user_id: i32,
    tx: &mut PgConnection,
) -> Result<Vec<String>, sqlx::Error> {
    let codes = new_recovery_codes();
    let hashes: Vec<String> = codes
        .iter()
        .map(|code| hash_secret(&normalize_recovery_code(code)))
        .collect();
    query_delete_recovery_codes(user_id, &mut *tx).await?;
    query_insert_recovery_codes(user_id, &hashes, &mut *tx).await?;
    Ok(codes)
}

async fn log_and_tell(
    env: &Env<'_>,
    caller: &Caller,
    kind: AccountEventKind,
    email: fn(&str, &str) -> Email,
    tx: &mut PgConnection,
) -> topcoat::Result<()> {
    record_event(env, &EventSource::of(caller), kind, json!({}), &mut *tx).await?;
    let sign_in_url = env.app.url(&AccountSection::TwoFactor.path());
    enqueue(
        &SendEmail(email(&caller.user.email, &sign_in_url)),
        &mut *tx,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
