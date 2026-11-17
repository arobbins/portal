use std::convert::Infallible;

use serde_json::json;

use crate::action::{ActionResult, Caller, authorize};
use crate::env::Env;
use crate::passkeys::queries::{
    query_decline_passkey_prompt, query_get_passkey_prompt_due, query_snooze_passkey_prompt,
};
use crate::passkeys::session::PasskeySession;
use crate::permission::Permission;
use crate::user::events::{AccountEventKind, EventSource, record_event};

pub const PROMPT_SNOOZE_DAYS: i32 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptAnswer {
    NotNow,
    Never,
}

pub async fn passkey_prompt_due(env: &Env<'_>, user_id: i32) -> Result<bool, sqlx::Error> {
    query_get_passkey_prompt_due(user_id, env.db()).await
}

pub async fn never_prompt_for_passkeys(user_id: i32, db: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    query_decline_passkey_prompt(user_id, db).await
}

pub async fn answer_passkey_prompt(
    env: &Env<'_>,
    session: &impl PasskeySession,
    caller: &Caller,
    answer: PromptAnswer,
) -> ActionResult<(), Infallible> {
    authorize!(caller.require(Permission::ManagePasskeys));
    let user_id = caller.user.id;
    let mut tx = env.db().begin().await?;
    let kind = match answer {
        PromptAnswer::NotNow => {
            query_snooze_passkey_prompt(user_id, PROMPT_SNOOZE_DAYS, &mut *tx).await?;
            AccountEventKind::PasskeyPromptSnoozed
        }
        PromptAnswer::Never => {
            query_decline_passkey_prompt(user_id, &mut *tx).await?;
            AccountEventKind::PasskeyPromptDeclined
        }
    };
    record_event(env, &EventSource::of(caller), kind, json!({}), &mut *tx).await?;
    tx.commit().await?;
    session.revoke_prompt().await?;

    Ok(Ok(()))
}
