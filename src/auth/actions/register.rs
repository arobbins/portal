use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{EMAIL_FIELD, INVALID_EMAIL};
use crate::action::{ActionProblem, Outcome, ProblemSpec, TooManyAttempts, fail};
use crate::auth::emails::account_exists_email;
use crate::auth::handoff::Handoff;
use crate::auth::links::send_verification_link_for;
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::routes::LOGIN_PATH;
use crate::time_zone::Zone;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::{AccountStatus, User};
use crate::user::name::{MAX_NAME_LENGTH, NameProblem, check_name};
use crate::user::queries::{
    NewUser, query_get_user_by_email, query_insert_user, query_update_user_time_zone,
};
use crate::utils::parse_email;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Start creating an account; a confirmation link is emailed.")]
pub struct Register {
    #[schemars(description = "Display name.", length(max = MAX_NAME_LENGTH))]
    pub name: String,
    #[schemars(description = "Email address to sign in with.")]
    pub email: String,
    #[serde(default)]
    #[schemars(
        description = "The browser's IANA time zone, e.g. Europe/London; unknown zones are ignored."
    )]
    pub time_zone: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "A Stripe price id chosen on the Pricing page; after setting a password the new account goes straight on to Checkout for it."
    )]
    pub price: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "An app's pending sign-in request (oar_…); after setting a password the new account goes on to approve it."
    )]
    pub authorize: Option<String>,
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
    fn spec(self) -> ProblemSpec {
        match self {
            Self::MissingDetails => {
                ProblemSpec::new("details_missing", "Please enter your name and email.")
            }
            Self::Name(problem) => problem.spec(),
            Self::InvalidEmail => {
                ProblemSpec::new("invalid_email", INVALID_EMAIL).on_field(EMAIL_FIELD)
            }
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

pub async fn register(env: &Env<'_>, input: &Register) -> Outcome<(), RegisterProblem> {
    if !env.limits().allow_email_request(env.ip).await? {
        return fail(RegisterProblem::TooManyAttempts);
    }
    let (name, email) = match valid_details(input) {
        Ok(details) => details,
        Err(problem) => return fail(problem),
    };
    if !env.limits().allow_email_to(&email).await? {
        tracing::warn!("registration email skipped: too many emails to this address");
        return Ok(Ok(()));
    }

    match query_get_user_by_email(&email, env.db()).await? {
        Some(user) => remind_existing_user(env, &user, handoff_payload(input).as_deref()).await?,
        None => {
            start_new_account(
                env,
                name,
                &email,
                known_zone(input),
                handoff_payload(input).as_deref(),
            )
            .await?
        }
    }
    Ok(Ok(()))
}

fn valid_details(input: &Register) -> Result<(&str, String), RegisterProblem> {
    if input.name.trim().is_empty() || input.email.trim().is_empty() {
        return Err(RegisterProblem::MissingDetails);
    }
    let name = check_name(&input.name).map_err(RegisterProblem::Name)?;
    let email = parse_email(&input.email).ok_or(RegisterProblem::InvalidEmail)?;
    Ok((name, email))
}

fn handoff_payload(input: &Register) -> Option<String> {
    Handoff::chosen(input.price.as_deref(), input.authorize.as_deref())
        .map(|handoff| handoff.value().to_string())
}

fn known_zone(input: &Register) -> Option<Zone> {
    input.time_zone.as_deref().and_then(Zone::parse)
}

async fn remind_existing_user(
    env: &Env<'_>,
    user: &User,
    price: Option<&str>,
) -> Result<(), sqlx::Error> {
    match user.status() {
        AccountStatus::Active { .. } => {
            let email = account_exists_email(&user.email, &env.app.url(LOGIN_PATH));
            enqueue(&SendEmail(email), env.db()).await
        }
        AccountStatus::AwaitingPassword => {
            send_verification_link_for(env.app, user.id, &user.email, price).await
        }
        AccountStatus::Disabled { .. } => Ok(()),
    }
}

async fn start_new_account(
    env: &Env<'_>,
    name: &str,
    email: &str,
    time_zone: Option<Zone>,
    price: Option<&str>,
) -> topcoat::Result<()> {
    let mut tx = env.db().begin().await?;
    let Some(user_id) = query_insert_user(&NewUser { name, email }, &mut *tx).await? else {
        return Ok(());
    };
    if let Some(zone) = time_zone {
        query_update_user_time_zone(user_id, zone.name(), &mut *tx).await?;
    }
    let source = EventSource::browser(user_id);
    record_event(
        env,
        &source,
        AccountEventKind::AccountCreated,
        json!({}),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(send_verification_link_for(env.app, user_id, email, price).await?)
}

#[cfg(test)]
mod tests;
