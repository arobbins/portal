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

impl AccountEventKind {
    pub fn parse(kind: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == kind)
    }
}

pub async fn query_list_recent_account_events(
    user_id: i32,
    limit: i64,
    db: impl PgExecutor<'_>,
) -> Result<Vec<PastEvent>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT kind, detail, ip, channel, created_at FROM account_events WHERE user_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2",
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
                via: EventChannel::parse(&row.channel),
                at: row.created_at,
            })
        })
        .collect())
}
