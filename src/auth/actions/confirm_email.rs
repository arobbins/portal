use serde_json::json;
use sqlx::PgConnection;
use topcoat::router::StatusCode;

use crate::action::{ActionProblem, Outcome, ProblemSpec, fail};
use crate::auth::emails::email_changed_email;
use crate::auth::links::TokenPurpose;
use crate::auth::queries::{
    LinkPayload, query_consume_user_token_payload, query_delete_user_tokens,
    query_get_valid_user_token_payload,
};
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::secret::hash_secret;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::User;
use crate::user::queries::{query_get_user_by_id, query_lock_user, query_update_user_email};

const PURPOSE: TokenPurpose = TokenPurpose::EmailChange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmEmailProblem {
    ExpiredLink,
    EmailTaken,
}

impl ActionProblem for ConfirmEmailProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::ExpiredLink => ProblemSpec::new(
                "link_expired",
                "This link is invalid or has already been used.",
            )
            .with_status(StatusCode::BAD_REQUEST),
            Self::EmailTaken => ProblemSpec::new(
                "email_taken",
                "This email address now belongs to another account, so your email wasn't changed.",
            )
            .with_status(StatusCode::CONFLICT),
        }
    }
}

pub async fn pending_email_change(env: &Env<'_>, token: &str) -> topcoat::Result<Option<String>> {
    let link = query_get_valid_user_token_payload(PURPOSE, &hash_secret(token), env.db()).await?;
    Ok(link.map(|link| link.payload))
}

pub async fn confirm_email_change(
    env: &Env<'_>,
    token: &str,
) -> Outcome<User, ConfirmEmailProblem> {
    let token_hash = hash_secret(token);
    let Some(found) = query_get_valid_user_token_payload(PURPOSE, &token_hash, env.db()).await?
    else {
        return fail(ConfirmEmailProblem::ExpiredLink);
    };
    let mut tx = env.db().begin().await?;
    query_lock_user(found.user_id, &mut *tx).await?;
    let Some(link) = query_consume_user_token_payload(PURPOSE, &token_hash, &mut *tx).await? else {
        return fail(ConfirmEmailProblem::ExpiredLink);
    };
    let Some((old_email, user)) = switch_email(&link, &mut tx).await? else {
        return fail(ConfirmEmailProblem::EmailTaken);
    };

    let notice = SendEmail(email_changed_email(&old_email, &link.payload));
    enqueue(&notice, &mut *tx).await?;
    let source = EventSource::browser(link.user_id);
    let kind = AccountEventKind::EmailChanged;
    record_event(env, &source, kind, json!({}), &mut *tx).await?;
    crate::billing::follow_email_change(link.user_id, &mut tx).await?;
    tx.commit().await?;

    Ok(Ok(user))
}

async fn switch_email(
    link: &LinkPayload,
    tx: &mut PgConnection,
) -> Result<Option<(String, User)>, sqlx::Error> {
    let Some(before) = query_get_user_by_id(link.user_id, &mut *tx).await? else {
        return Ok(None);
    };
    let Some(user) = query_update_user_email(link.user_id, &link.payload, &mut *tx).await? else {
        return Ok(None);
    };
    query_delete_user_tokens(link.user_id, PURPOSE, &mut *tx).await?;
    query_delete_user_tokens(link.user_id, TokenPurpose::PasswordReset, &mut *tx).await?;
    Ok(Some((before.email, user)))
}
