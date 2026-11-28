use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgConnection;
use topcoat::router::StatusCode;

use super::change_password::announce_password_change;
use crate::action::{ActionProblem, Outcome, ProblemSpec, TooManyAttempts, fail};
use crate::api_tokens::lifecycle::revoke_all_for_user;
use crate::auth::links::TokenPurpose;
use crate::auth::password::{
    MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH, PasswordProblem, check_new_password, hash_password,
};
use crate::auth::queries::{query_consume_user_token, query_delete_user_tokens};
use crate::env::Env;
use crate::oauth::lifecycle::disconnect_all_for_user;
use crate::secret::hash_secret;
use crate::two_factor::actions::{CODE_FIELD, second_factor_matches};
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::User;
use crate::user::queries::{
    query_lock_user, query_set_password_and_verify_user, query_update_password,
};

mod lookup;

use lookup::{asks_for_code, link_holder};
pub use lookup::{link_is_valid, link_needs_code, link_payload};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Choose a new password.")]
pub struct NewPassword {
    #[schemars(
        description = "The new password.",
        length(min = MIN_PASSWORD_LENGTH, max = MAX_PASSWORD_LENGTH)
    )]
    pub password: String,
    #[schemars(description = "The new password again.")]
    pub confirm_password: String,
    #[serde(default)]
    #[schemars(
        description = "A code from the authenticator app or a recovery code, when two-factor is on.",
        length(max = 32)
    )]
    pub code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPasswordProblem {
    ExpiredLink,
    Password(PasswordProblem),
    WrongCode,
    TooManyCodeAttempts,
    TooManyAttempts,
}

impl ActionProblem for LinkPasswordProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::ExpiredLink => ProblemSpec::new(
                "link_expired",
                "This link is invalid or has already been used.",
            )
            .with_status(StatusCode::BAD_REQUEST),
            Self::Password(problem) => problem.spec(),
            Self::WrongCode => ProblemSpec::new(
                "wrong_code",
                "That code didn't work, so this link has been used up. Request a new link to try again.",
            )
            .on_field(CODE_FIELD)
            .with_status(StatusCode::BAD_REQUEST),
            Self::TooManyCodeAttempts | Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

pub async fn set_password_from_link(
    env: &Env<'_>,
    purpose: TokenPurpose,
    token: &str,
    input: &NewPassword,
) -> Outcome<User, LinkPasswordProblem> {
    if !env.limits().allow_password_submission(env.ip).await? {
        return fail(LinkPasswordProblem::TooManyAttempts);
    }
    let Some(holder) = link_holder(env, purpose, token).await? else {
        return fail(LinkPasswordProblem::ExpiredLink);
    };
    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return fail(LinkPasswordProblem::Password(problem));
    }
    let needs_code = asks_for_code(env, purpose, &holder).await?;
    if needs_code
        && !env
            .limits()
            .take_second_factor_attempt(&holder.email)
            .await?
    {
        return fail(LinkPasswordProblem::TooManyCodeAttempts);
    }

    let password_hash = hash_password(&input.password)?;
    let code = needs_code.then_some(input.code.as_str());
    let outcome = use_link(env, purpose, token, &holder, &password_hash, code).await?;
    if needs_code && outcome.is_ok() {
        env.limits()
            .clear_failed_second_factors(&holder.email)
            .await?;
    }
    Ok(outcome)
}

async fn use_link(
    env: &Env<'_>,
    purpose: TokenPurpose,
    token: &str,
    holder: &User,
    password_hash: &str,
    code: Option<&str>,
) -> Outcome<User, LinkPasswordProblem> {
    let mut tx = env.db().begin().await?;
    query_lock_user(holder.id, &mut *tx).await?;
    let Some(user_id) = consume_link(purpose, token, &mut tx).await? else {
        return fail(LinkPasswordProblem::ExpiredLink);
    };
    if let Some(code) = code
        && !second_factor_matches(env, holder, code, &mut tx).await?
    {
        tx.commit().await?;
        return fail(LinkPasswordProblem::WrongCode);
    }
    let applied = apply_password(env, purpose, user_id, password_hash, &mut tx).await?;
    tx.commit().await?;
    match applied {
        Some(user) => Ok(Ok(user)),
        None => fail(LinkPasswordProblem::ExpiredLink),
    }
}

async fn consume_link(
    purpose: TokenPurpose,
    token: &str,
    tx: &mut PgConnection,
) -> Result<Option<i32>, sqlx::Error> {
    let Some(user_id) = query_consume_user_token(purpose, &hash_secret(token), &mut *tx).await?
    else {
        return Ok(None);
    };
    query_delete_user_tokens(user_id, purpose, &mut *tx).await?;
    Ok(Some(user_id))
}

async fn apply_password(
    env: &Env<'_>,
    purpose: TokenPurpose,
    user_id: i32,
    password_hash: &str,
    tx: &mut PgConnection,
) -> topcoat::Result<Option<User>> {
    let source = EventSource::browser(user_id);
    match purpose {
        TokenPurpose::EmailVerification => {
            let Some(user) =
                query_set_password_and_verify_user(user_id, password_hash, &mut *tx).await?
            else {
                return Ok(None);
            };
            record_event(
                env,
                &source,
                AccountEventKind::EmailVerified,
                json!({}),
                &mut *tx,
            )
            .await?;
            Ok(Some(user))
        }
        TokenPurpose::PasswordReset => reset_password(env, user_id, password_hash, tx).await,
        TokenPurpose::EmailChange => Ok(None),
    }
}

async fn reset_password(
    env: &Env<'_>,
    user_id: i32,
    password_hash: &str,
    tx: &mut PgConnection,
) -> topcoat::Result<Option<User>> {
    let Some(user) = query_update_password(user_id, password_hash, &mut *tx).await? else {
        return Ok(None);
    };
    let revoked = revoke_access(user_id, &mut *tx).await?;
    let apps = disconnect_all_for_user(user_id, &mut *tx).await?;
    let detail = json!({ "api_tokens_revoked": revoked, "apps_disconnected": apps });
    let source = EventSource::browser(user_id);
    record_event(
        env,
        &source,
        AccountEventKind::PasswordReset,
        detail,
        &mut *tx,
    )
    .await?;
    announce_password_change(env, &user.email, &mut *tx).await?;
    Ok(Some(user))
}

async fn revoke_access(user_id: i32, tx: &mut PgConnection) -> Result<u64, sqlx::Error> {
    query_delete_user_tokens(user_id, TokenPurpose::EmailChange, &mut *tx).await?;
    revoke_all_for_user(user_id, &mut *tx).await
}

#[cfg(test)]
mod tests;
