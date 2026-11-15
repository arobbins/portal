use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::INVALID_EMAIL;
use crate::action::{
    ActionProblem, ActionResult, Caller, Outcome, ProblemSpec, TooManyAttempts, authorize, fail,
};
use crate::auth::emails::{email_change_email, email_in_use_email};
use crate::auth::links::{NewLink, TokenPurpose, email_user_link};
use crate::auth::reauth::{PasskeyProof, ReauthProblem, check_reauth};
use crate::env::{App, Env};
use crate::jobs::{Job, JobError, enqueue};
use crate::mail::SendEmail;
use crate::permission::Permission;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::queries::{query_get_user_by_email, query_get_user_by_id};
use crate::utils::parse_email;

pub const NEW_EMAIL_FIELD: &str = "new_email";

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    description = "Ask to sign in with a different email address. A link is sent to the new address; the email changes only once it is confirmed."
)]
pub struct ChangeEmail {
    #[schemars(description = "The email address to sign in with from now on.")]
    pub new_email: String,
    #[serde(default)]
    #[schemars(
        description = "The current password; leave it empty right after confirming with a passkey."
    )]
    pub current_password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeEmailProblem {
    InvalidEmail,
    SameEmail,
    TooManyAttempts,
    Reauth(ReauthProblem),
}

impl ActionProblem for ChangeEmailProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::InvalidEmail => {
                ProblemSpec::new("invalid_email", INVALID_EMAIL).on_field(NEW_EMAIL_FIELD)
            }
            Self::SameEmail => ProblemSpec::new("same_email", "That's already your email address.")
                .on_field(NEW_EMAIL_FIELD),
            Self::TooManyAttempts => TooManyAttempts.spec(),
            Self::Reauth(problem) => problem.spec(),
        }
    }
}

pub async fn request_email_change(
    env: &Env<'_>,
    caller: &Caller,
    input: &ChangeEmail,
    passkey: Option<PasskeyProof>,
) -> ActionResult<(), ChangeEmailProblem> {
    authorize!(caller.require(Permission::ManageEmail));
    let new_email = match check_request(env, caller, input, passkey).await? {
        Ok(new_email) => new_email,
        Err(problem) => return fail(problem),
    };

    let mut tx = env.db().begin().await?;
    if env.limits().allow_email_to(&new_email).await? {
        let job = EmailChangeJob {
            user_id: caller.user.id,
            new_email,
        };
        enqueue(&job, &mut *tx).await?;
    } else {
        tracing::warn!("email change link skipped: too many emails to this address");
    }
    let source = EventSource::of(caller);
    let kind = AccountEventKind::EmailChangeRequested;
    record_event(env, &source, kind, json!({}), &mut *tx).await?;
    tx.commit().await?;

    Ok(Ok(()))
}

async fn check_request(
    env: &Env<'_>,
    caller: &Caller,
    input: &ChangeEmail,
    passkey: Option<PasskeyProof>,
) -> Outcome<String, ChangeEmailProblem> {
    let Some(new_email) = parse_email(&input.new_email) else {
        return fail(ChangeEmailProblem::InvalidEmail);
    };
    if new_email == caller.user.email {
        return fail(ChangeEmailProblem::SameEmail);
    }
    if !env.limits().allow_email_request(env.ip).await? {
        return fail(ChangeEmailProblem::TooManyAttempts);
    }
    let password = &input.current_password;
    if let Err(problem) = check_reauth(env, &caller.user, password, passkey).await? {
        return fail(ChangeEmailProblem::Reauth(problem));
    }

    Ok(Ok(new_email))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailChangeJob {
    pub user_id: i32,
    pub new_email: String,
}

impl Job for EmailChangeJob {
    const KIND: &'static str = "send_email_change";

    async fn run(self, app: &App) -> Result<(), JobError> {
        if query_get_user_by_id(self.user_id, &app.db).await?.is_none() {
            return Ok(());
        }
        if query_get_user_by_email(&self.new_email, &app.db)
            .await?
            .is_some()
        {
            let notice = SendEmail(email_in_use_email(&self.new_email));
            return Ok(enqueue(&notice, &app.db).await?);
        }
        let link = NewLink {
            user_id: self.user_id,
            purpose: TokenPurpose::EmailChange,
            payload: Some(&self.new_email),
            email: &|url| email_change_email(&self.new_email, url),
        };
        Ok(email_user_link(app, link).await?)
    }
}

#[cfg(test)]
mod tests;
