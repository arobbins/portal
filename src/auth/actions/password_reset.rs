use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{EMAIL_FIELD, INVALID_EMAIL};
use crate::action::{ActionProblem, Outcome, ProblemSpec, TooManyAttempts, fail};
use crate::auth::emails::password_reset_email;
use crate::auth::links::{NewLink, TokenPurpose, email_user_link, send_verification_link};
use crate::env::{App, Env};
use crate::jobs::{Job, JobError, enqueue};
use crate::user::model::AccountStatus;
use crate::user::queries::query_get_user_by_email;
use crate::utils::parse_email;

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
    fn spec(self) -> ProblemSpec {
        match self {
            Self::InvalidEmail => {
                ProblemSpec::new("invalid_email", INVALID_EMAIL).on_field(EMAIL_FIELD)
            }
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

pub async fn request_password_reset(
    env: &Env<'_>,
    input: &RequestPasswordReset,
) -> Outcome<(), ResetRequestProblem> {
    if !env.limits().allow_email_request(env.ip).await? {
        return fail(ResetRequestProblem::TooManyAttempts);
    }
    let Some(email) = parse_email(&input.email) else {
        return fail(ResetRequestProblem::InvalidEmail);
    };
    if !env.limits().allow_email_to(&email).await? {
        tracing::warn!("password reset email skipped: too many emails to this address");
        return Ok(Ok(()));
    }

    enqueue(&PasswordResetJob { email }, env.db()).await?;
    Ok(Ok(()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordResetJob {
    pub email: String,
}

impl Job for PasswordResetJob {
    const KIND: &'static str = "send_password_reset";

    async fn run(self, app: &App) -> Result<(), JobError> {
        let Some(user) = query_get_user_by_email(&self.email, &app.db).await? else {
            return Ok(());
        };
        match user.status() {
            AccountStatus::Active { .. } => {
                let link = NewLink {
                    user_id: user.id,
                    purpose: TokenPurpose::PasswordReset,
                    email: &|url| password_reset_email(&user.email, url),
                };
                Ok(email_user_link(app, link).await?)
            }
            AccountStatus::AwaitingPassword => {
                Ok(send_verification_link(app, user.id, &user.email).await?)
            }
        }
    }
}
