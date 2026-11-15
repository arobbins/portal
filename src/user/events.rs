use std::net::IpAddr;

use serde_json::Value;
use sqlx::PgExecutor;
use topcoat::Result;

use crate::action::{Caller, Channel};
use crate::env::Env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountEventKind {
    AccountCreated,
    ApiTokenCreated,
    ApiTokenRevoked,
    EmailVerified,
    PasskeyAdded,
    PasskeyPromptDeclined,
    PasskeyPromptSnoozed,
    PasskeyRemoved,
    PasskeyRenamed,
    PasswordChanged,
    PasswordReset,
    ProfileUpdated,
}

impl AccountEventKind {
    #[cfg(test)]
    pub const ALL: [Self; 12] = [
        Self::AccountCreated,
        Self::ApiTokenCreated,
        Self::ApiTokenRevoked,
        Self::EmailVerified,
        Self::PasskeyAdded,
        Self::PasskeyPromptDeclined,
        Self::PasskeyPromptSnoozed,
        Self::PasskeyRemoved,
        Self::PasskeyRenamed,
        Self::PasswordChanged,
        Self::PasswordReset,
        Self::ProfileUpdated,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AccountCreated => "account_created",
            Self::ApiTokenCreated => "api_token_created",
            Self::ApiTokenRevoked => "api_token_revoked",
            Self::EmailVerified => "email_verified",
            Self::PasskeyAdded => "passkey_added",
            Self::PasskeyPromptDeclined => "passkey_prompt_declined",
            Self::PasskeyPromptSnoozed => "passkey_prompt_snoozed",
            Self::PasskeyRemoved => "passkey_removed",
            Self::PasskeyRenamed => "passkey_renamed",
            Self::PasswordChanged => "password_changed",
            Self::PasswordReset => "password_reset",
            Self::ProfileUpdated => "profile_updated",
        }
    }
}

pub struct EventSource {
    pub user_id: i32,
    pub channel: Channel,
}

impl EventSource {
    pub fn of(caller: &Caller) -> Self {
        Self {
            user_id: caller.user.id,
            channel: caller.via.clone(),
        }
    }

    pub fn browser(user_id: i32) -> Self {
        Self {
            user_id,
            channel: Channel::Browser,
        }
    }
}

pub struct NewAccountEvent<'a> {
    pub source: &'a EventSource,
    pub kind: AccountEventKind,
    pub detail: &'a Value,
    pub ip: Option<IpAddr>,
}

pub async fn record_event(
    env: &Env<'_>,
    source: &EventSource,
    kind: AccountEventKind,
    detail: Value,
    db: impl PgExecutor<'_>,
) -> Result<()> {
    query_insert_account_event(
        &NewAccountEvent {
            source,
            kind,
            detail: &detail,
            ip: env.ip,
        },
        db,
    )
    .await?;

    Ok(())
}

pub async fn query_insert_account_event(
    event: &NewAccountEvent<'_>,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO account_events (user_id, kind, detail, ip, channel, api_token_id) VALUES ($1, $2, $3, $4, $5, $6)",
        event.source.user_id,
        event.kind.as_str(),
        event.detail,
        event.ip.map(|ip| ip.to_string()),
        event.source.channel.as_str(),
        event.source.channel.api_token_id(),
    )
    .execute(db)
    .await?;

    Ok(())
}

pub const EVENT_RETENTION: time::Duration = time::Duration::days(395);

pub async fn delete_old_events(app: &crate::env::App) -> Result<u64, sqlx::Error> {
    query_delete_old_account_events(app.clock.now() - EVENT_RETENTION, &app.db).await
}

async fn query_delete_old_account_events(
    cutoff: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM account_events WHERE created_at < $1", cutoff)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::queries::{NewUser, query_insert_user};
    use serde_json::json;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn events_record_which_token_made_the_change(db: PgPool) {
        let user_id = query_insert_user(
            &NewUser {
                name: "Logged",
                email: "logged@example.test",
            },
            &db,
        )
        .await
        .unwrap()
        .unwrap();
        let channels = [
            Channel::Browser,
            Channel::ApiToken {
                id: 42,
                scopes: vec![],
            },
        ];
        for channel in channels {
            query_insert_account_event(
                &NewAccountEvent {
                    source: &EventSource { user_id, channel },
                    kind: AccountEventKind::ProfileUpdated,
                    detail: &json!({ "fields": ["name"] }),
                    ip: None,
                },
                &db,
            )
            .await
            .unwrap();
        }

        let rows = sqlx::query!(
            "SELECT channel, api_token_id, detail FROM account_events WHERE user_id = $1 ORDER BY id",
            user_id,
        )
        .fetch_all(&db)
        .await
        .unwrap();
        let logged: Vec<(String, Option<i32>, Value)> = rows
            .into_iter()
            .map(|row| (row.channel, row.api_token_id, row.detail))
            .collect();

        assert_eq!(
            logged,
            vec![
                ("browser".to_string(), None, json!({ "fields": ["name"] })),
                (
                    "api_token".to_string(),
                    Some(42),
                    json!({ "fields": ["name"] })
                )
            ]
        );
    }

    #[sqlx::test]
    async fn the_database_accepts_every_event_kind(db: PgPool) {
        let user_id = query_insert_user(
            &NewUser {
                name: "Kinds",
                email: "kinds@example.test",
            },
            &db,
        )
        .await
        .unwrap()
        .unwrap();
        let source = EventSource::browser(user_id);

        for kind in AccountEventKind::ALL {
            query_insert_account_event(
                &NewAccountEvent {
                    source: &source,
                    kind,
                    detail: &json!({}),
                    ip: None,
                },
                &db,
            )
            .await
            .unwrap();
        }
    }
}
