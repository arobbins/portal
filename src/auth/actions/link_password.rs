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
use crate::auth::queries::{
    query_consume_user_token, query_delete_user_tokens, query_get_valid_user_token,
};
use crate::env::Env;
use crate::secret::hash_secret;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::User;
use crate::user::queries::{query_set_password_and_verify_user, query_update_password};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPasswordProblem {
    ExpiredLink,
    Password(PasswordProblem),
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
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

pub async fn link_is_valid(
    env: &Env<'_>,
    purpose: TokenPurpose,
    token: &str,
) -> topcoat::Result<bool> {
    let user_id = query_get_valid_user_token(purpose, &hash_secret(token), env.db()).await?;
    Ok(user_id.is_some())
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
    if !link_is_valid(env, purpose, token).await? {
        return fail(LinkPasswordProblem::ExpiredLink);
    }
    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return fail(LinkPasswordProblem::Password(problem));
    }

    let password_hash = hash_password(&input.password)?;
    let mut tx = env.db().begin().await?;
    let Some(user_id) = consume_link(purpose, token, &mut tx).await? else {
        return fail(LinkPasswordProblem::ExpiredLink);
    };
    let Some(user) = apply_password(env, purpose, user_id, &password_hash, &mut tx).await? else {
        return fail(LinkPasswordProblem::ExpiredLink);
    };
    tx.commit().await?;

    Ok(Ok(user))
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
            let user = query_set_password_and_verify_user(user_id, password_hash, &mut *tx).await?;
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
    let detail = json!({ "api_tokens_revoked": revoked });
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
