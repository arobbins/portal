use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;

use super::events::{AccountEventKind, EventChannel};

#[derive(Debug, Clone, PartialEq)]
pub struct PastEvent {
    pub kind: AccountEventKind,
    pub detail: Value,
    pub ip: Option<String>,
    pub via: EventChannel,
    pub at: OffsetDateTime,
}

pub async fn query_list_recent_account_events(
    user_id: i32,
    limit: i64,
    db: impl PgExecutor<'_>,
) -> Result<Vec<PastEvent>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT kind, detail, ip, channel, api_token_id, created_at FROM account_events WHERE user_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2",
        user_id,
        limit,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            Some(PastEvent {
                kind: AccountEventKind::parse(&row.kind)?,
                detail: row.detail,
                ip: row.ip,
                via: EventChannel::parse(&row.channel, row.api_token_id)?,
                at: row.created_at,
            })
        })
        .collect())
}

pub async fn query_last_account_event_at(
    user_id: i32,
    kinds: &[AccountEventKind],
    db: impl PgExecutor<'_>,
) -> Result<Option<OffsetDateTime>, sqlx::Error> {
    let kinds: Vec<String> = kinds.iter().map(|kind| kind.as_str().to_string()).collect();
    sqlx::query_scalar!(
        "SELECT max(created_at) FROM account_events WHERE user_id = $1 AND kind = ANY($2)",
        user_id,
        &kinds,
    )
    .fetch_one(db)
    .await
}
