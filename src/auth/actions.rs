use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, request::client_ip},
};

use crate::action::{
    ActionProblem, ActionResult, Caller, Outcome, TooManyAttempts, authorize, fail,
};
use crate::api_tokens::verify::revoke_all_for_user;
use crate::auth::emails::{
    account_exists_email, password_reset_email, register_verification_email,
};
use crate::auth::links::{TokenPurpose, email_user_link};
use crate::auth::password::{
    CurrentPasswordProblem, MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH, PasswordProblem,
    check_current_password, check_new_password, hash_password, password_matches,
};
use crate::auth::queries::{
    query_consume_user_token, query_delete_user_tokens, query_get_valid_user_token,
};
use crate::config::Config;
use crate::jobs::{Job, enqueue};
use crate::permission::Permission;
use crate::routes::login_url;
use crate::secret::hash_secret;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::{AccountStatus, User};
use crate::user::name::{MAX_NAME_LENGTH, NameProblem, check_name};
use crate::user::queries::{
    NewUser, query_get_user_by_email, query_insert_user, query_set_password_and_verify_user,
    query_update_password,
};
use crate::utils::parse_email;
use crate::web::{app_url, config, db, rate_limits};

pub const EMAIL_FIELD: &str = "email";

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Sign in with an email and password.")]
pub struct SignIn {
    #[schemars(description = "The account's email address.")]
    pub email: String,
    #[schemars(description = "The account's password.")]
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInProblem {
    InvalidCredentials,
    TooManyAttempts,
}

impl ActionProblem for SignInProblem {
    fn code(self) -> &'static str {
        match self {
            Self::InvalidCredentials => "invalid_credentials",
            Self::TooManyAttempts => TooManyAttempts.code(),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::InvalidCredentials => "Invalid email or password.",
            Self::TooManyAttempts => TooManyAttempts.message(),
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::InvalidCredentials => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyAttempts => TooManyAttempts.status(),
        }
    }
}

pub async fn sign_in_with_password(cx: &Cx, input: &SignIn) -> Outcome<User, SignInProblem> {
    let limits = rate_limits(cx);
    let ip = client_ip(cx);
    if !limits.allow_password_submission(ip).await? {
        return fail(SignInProblem::TooManyAttempts);
    }
    let Some(email) = parse_email(&input.email) else {
        return fail(SignInProblem::InvalidCredentials);
    };
    if limits.login_blocked(&email, ip).await? {
        return fail(SignInProblem::TooManyAttempts);
    }

    let user = query_get_user_by_email(&email, db(cx)).await?;
    let hash = user.as_ref().and_then(|user| match user.status() {
        AccountStatus::Active { password_hash } => Some(password_hash),
        AccountStatus::AwaitingPassword => None,
    });
    let matches = password_matches(hash, &input.password);
    match user.filter(|_| matches) {
        Some(user) => {
            limits.clear_failed_logins(&email, ip).await?;
            Ok(Ok(user))
        }
        None => {
            limits.record_failed_login(&email, ip).await?;
            fail(SignInProblem::InvalidCredentials)
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Start creating an account; a confirmation link is emailed.")]
pub struct Register {
    #[schemars(description = "Display name.", length(max = MAX_NAME_LENGTH))]
    pub name: String,
    #[schemars(description = "Email address to sign in with.")]
    pub email: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterProblem {
    MissingDetails,
    Name(NameProblem),
    InvalidEmail,
    TooManyAttempts,
}

impl RegisterProblem {
    pub fn invalid_name(self) -> bool {
        matches!(self, Self::MissingDetails | Self::Name(_))
    }

    pub fn invalid_email(self) -> bool {
        matches!(self, Self::MissingDetails | Self::InvalidEmail)
    }
}

impl ActionProblem for RegisterProblem {
    fn code(self) -> &'static str {
        match self {
            Self::MissingDetails => "details_missing",
            Self::Name(problem) => problem.code(),
            Self::InvalidEmail => "invalid_email",
            Self::TooManyAttempts => TooManyAttempts.code(),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::MissingDetails => "Please enter your name and email.",
            Self::Name(problem) => problem.message(),
            Self::InvalidEmail => INVALID_EMAIL,
            Self::TooManyAttempts => TooManyAttempts.message(),
        }
    }

    fn field(self) -> Option<&'static str> {
        match self {
            Self::Name(problem) => problem.field(),
            Self::InvalidEmail => Some(EMAIL_FIELD),
            Self::MissingDetails | Self::TooManyAttempts => None,
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::TooManyAttempts => TooManyAttempts.status(),
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

const INVALID_EMAIL: &str = "Please enter a valid email address.";

pub async fn register(cx: &Cx, input: &Register) -> Outcome<(), RegisterProblem> {
    if !rate_limits(cx).allow_email_request(client_ip(cx)).await? {
        return fail(RegisterProblem::TooManyAttempts);
    }
    if input.name.trim().is_empty() || input.email.trim().is_empty() {
        return fail(RegisterProblem::MissingDetails);
    }
    let name = match check_name(&input.name) {
        Ok(name) => name,
        Err(problem) => return fail(RegisterProblem::Name(problem)),
    };
    let Some(email) = parse_email(&input.email) else {
        return fail(RegisterProblem::InvalidEmail);
    };
    if !rate_limits(cx).allow_email_to(&email).await? {
        tracing::warn!("registration email skipped: too many emails to this address");
        return Ok(Ok(()));
    }

    let existing = query_get_user_by_email(&email, db(cx)).await?;
    match existing.as_ref().map(|user| (user, user.status())) {
        Some((user, AccountStatus::Active { .. })) => {
            let login_url = app_url(cx, &login_url(cx));
            let email = account_exists_email(&user.email, &login_url);
            enqueue(&Job::SendEmail(email), db(cx)).await?;
        }
        Some((user, AccountStatus::AwaitingPassword)) => {
            send_verification_link(db(cx), config(cx), user.id, &user.email).await?;
        }
        None => {
            if let Some(user_id) = create_user(cx, name, &email).await? {
                send_verification_link(db(cx), config(cx), user_id, &email).await?;
            }
        }
    }

    Ok(Ok(()))
}

async fn create_user(cx: &Cx, name: &str, email: &str) -> Result<Option<i32>> {
    let mut tx = db(cx).begin().await?;
    let Some(user_id) = query_insert_user(&NewUser { name, email }, &mut *tx).await? else {
        return Ok(None);
    };
    record_event(
        cx,
        &EventSource::browser(user_id),
        AccountEventKind::AccountCreated,
        json!({}),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Some(user_id))
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Ask for a password reset link.")]
pub struct RequestPasswordReset {
    #[schemars(description = "The account's email address.")]
    pub email: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetRequestProblem {
    InvalidEmail,
    TooManyAttempts,
}

impl ActionProblem for ResetRequestProblem {
    fn code(self) -> &'static str {
        match self {
            Self::InvalidEmail => "invalid_email",
            Self::TooManyAttempts => TooManyAttempts.code(),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::InvalidEmail => INVALID_EMAIL,
            Self::TooManyAttempts => TooManyAttempts.message(),
        }
    }

    fn field(self) -> Option<&'static str> {
        match self {
            Self::InvalidEmail => Some(EMAIL_FIELD),
            Self::TooManyAttempts => None,
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::InvalidEmail => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyAttempts => TooManyAttempts.status(),
        }
    }
}

pub async fn request_password_reset(
    cx: &Cx,
    input: &RequestPasswordReset,
) -> Outcome<(), ResetRequestProblem> {
    if !rate_limits(cx).allow_email_request(client_ip(cx)).await? {
        return fail(ResetRequestProblem::TooManyAttempts);
    }
    let Some(email) = parse_email(&input.email) else {
        return fail(ResetRequestProblem::InvalidEmail);
    };
    if !rate_limits(cx).allow_email_to(&email).await? {
        tracing::warn!("password reset email skipped: too many emails to this address");
        return Ok(Ok(()));
    }

    enqueue(&Job::SendPasswordReset { email }, db(cx)).await?;

    Ok(Ok(()))
}

pub async fn send_reset_link(db: &PgPool, config: &Config, email: &str) -> Result<(), sqlx::Error> {
    let Some(user) = query_get_user_by_email(email, db).await? else {
        return Ok(());
    };

    match user.status() {
        AccountStatus::Active { .. } => {
            email_user_link(db, config, user.id, TokenPurpose::PasswordReset, |url| {
                password_reset_email(&user.email, url)
            })
            .await
        }
        AccountStatus::AwaitingPassword => {
            send_verification_link(db, config, user.id, &user.email).await
        }
    }
}

async fn send_verification_link(
    db: &PgPool,
    config: &Config,
    user_id: i32,
    email: &str,
) -> Result<(), sqlx::Error> {
    email_user_link(
        db,
        config,
        user_id,
        TokenPurpose::EmailVerification,
        |url| register_verification_email(email, url),
    )
    .await
}

pub async fn link_is_valid(cx: &Cx, purpose: TokenPurpose, token: &str) -> Result<bool> {
    Ok(
        query_get_valid_user_token(purpose, &hash_secret(token), db(cx))
            .await?
            .is_some(),
    )
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Choose a new password.")]
pub struct NewPassword {
    #[schemars(description = "The new password.", length(min = MIN_PASSWORD_LENGTH, max = MAX_PASSWORD_LENGTH))]
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
    fn code(self) -> &'static str {
        match self {
            Self::ExpiredLink => "link_expired",
            Self::Password(problem) => problem.code(),
            Self::TooManyAttempts => TooManyAttempts.code(),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::ExpiredLink => "This link is invalid or has already been used.",
            Self::Password(problem) => problem.message(),
            Self::TooManyAttempts => TooManyAttempts.message(),
        }
    }

    fn field(self) -> Option<&'static str> {
        match self {
            Self::Password(problem) => problem.field(),
            Self::ExpiredLink | Self::TooManyAttempts => None,
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::ExpiredLink => StatusCode::BAD_REQUEST,
            Self::Password(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyAttempts => TooManyAttempts.status(),
        }
    }
}

pub async fn set_password_from_link(
    cx: &Cx,
    purpose: TokenPurpose,
    token: &str,
    input: &NewPassword,
) -> Outcome<User, LinkPasswordProblem> {
    if !rate_limits(cx)
        .allow_password_submission(client_ip(cx))
        .await?
    {
        return fail(LinkPasswordProblem::TooManyAttempts);
    }
    if !link_is_valid(cx, purpose, token).await? {
        return fail(LinkPasswordProblem::ExpiredLink);
    }
    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return fail(LinkPasswordProblem::Password(problem));
    }

    let password_hash = hash_password(&input.password)?;
    let mut tx = db(cx).begin().await?;
    let Some(user_id) = query_consume_user_token(purpose, &hash_secret(token), &mut *tx).await?
    else {
        return fail(LinkPasswordProblem::ExpiredLink);
    };
    query_delete_user_tokens(user_id, purpose, &mut *tx).await?;

    let source = EventSource::browser(user_id);
    let user = match purpose {
        TokenPurpose::EmailVerification => {
            let user =
                query_set_password_and_verify_user(user_id, &password_hash, &mut *tx).await?;
            record_event(
                cx,
                &source,
                AccountEventKind::EmailVerified,
                json!({}),
                &mut *tx,
            )
            .await?;
            user
        }
        TokenPurpose::PasswordReset => {
            let Some(user) = query_update_password(user_id, &password_hash, &mut *tx).await? else {
                return fail(LinkPasswordProblem::ExpiredLink);
            };
            let revoked = revoke_all_for_user(user_id, &mut *tx).await?;
            record_event(
                cx,
                &source,
                AccountEventKind::PasswordReset,
                json!({ "api_tokens_revoked": revoked }),
                &mut *tx,
            )
            .await?;
            user
        }
    };
    tx.commit().await?;

    Ok(Ok(user))
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Change the password while signed in.")]
pub struct ChangePassword {
    #[schemars(description = "The current password.")]
    pub current_password: String,
    #[schemars(description = "The new password.", length(min = MIN_PASSWORD_LENGTH, max = MAX_PASSWORD_LENGTH))]
    pub password: String,
    #[schemars(description = "The new password again.")]
    pub confirm_password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangePasswordProblem {
    CurrentPassword(CurrentPasswordProblem),
    NewPassword(PasswordProblem),
}

impl ActionProblem for ChangePasswordProblem {
    fn code(self) -> &'static str {
        match self {
            Self::CurrentPassword(problem) => problem.code(),
            Self::NewPassword(problem) => problem.code(),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::CurrentPassword(problem) => problem.message(),
            Self::NewPassword(problem) => problem.message(),
        }
    }

    fn field(self) -> Option<&'static str> {
        match self {
            Self::CurrentPassword(problem) => problem.field(),
            Self::NewPassword(problem) => problem.field(),
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::CurrentPassword(problem) => problem.status(),
            Self::NewPassword(problem) => problem.status(),
        }
    }
}

pub async fn change_password(
    cx: &Cx,
    caller: &Caller,
    input: &ChangePassword,
) -> ActionResult<User, ChangePasswordProblem> {
    authorize!(caller.require(Permission::ManagePassword));
    if let Err(problem) = check_current_password(cx, &caller.user, &input.current_password).await? {
        return fail(ChangePasswordProblem::CurrentPassword(problem));
    }
    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return fail(ChangePasswordProblem::NewPassword(problem));
    }

    let password_hash = hash_password(&input.password)?;
    let mut tx = db(cx).begin().await?;
    let Some(updated) = query_update_password(caller.user.id, &password_hash, &mut *tx).await?
    else {
        return Err(topcoat::Error::msg("signed-in user is not active"));
    };
    record_event(
        cx,
        &EventSource::of(caller),
        AccountEventKind::PasswordChanged,
        json!({}),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(updated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, Denied};
    use crate::api::scope::ApiScope;
    use std::sync::Arc;

    #[test]
    fn every_problem_with_a_field_names_a_real_input() {
        let register_fields = ["name", "email"];
        for problem in [
            RegisterProblem::Name(NameProblem::Missing),
            RegisterProblem::InvalidEmail,
        ] {
            assert!(register_fields.contains(&problem.field().unwrap()));
        }
        let password_fields = ["current_password", "password", "confirm_password"];
        for problem in [
            ChangePasswordProblem::CurrentPassword(CurrentPasswordProblem::Wrong),
            ChangePasswordProblem::NewPassword(PasswordProblem::TooShort),
            ChangePasswordProblem::NewPassword(PasswordProblem::Mismatch),
        ] {
            assert!(password_fields.contains(&problem.field().unwrap()));
        }
    }

    #[sqlx::test]
    async fn an_api_token_can_never_change_the_password(db: PgPool) {
        let cx = Cx::new().with(db.clone());
        let caller = Caller::api_token(
            Arc::new(User::default_for_tests()),
            1,
            ApiScope::ALL.to_vec(),
        );
        let input = ChangePassword {
            current_password: "hash".to_string(),
            password: "a-new-password".to_string(),
            confirm_password: "a-new-password".to_string(),
        };

        let outcome = change_password(&cx, &caller, &input).await.unwrap();

        assert_eq!(
            outcome.unwrap_err(),
            ActionError::Denied(Denied::BrowserOnly)
        );
    }
}
