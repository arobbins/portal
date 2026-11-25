use std::net::IpAddr;

use serde_json::Value;
use sqlx::PgExecutor;
use topcoat::Result;

pub use super::event_source::{EventChannel, EventSource};
use crate::env::Env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountEventKind {
    AccountCreated,
    ApiTokenCreated,
    ApiTokenRevoked,
    ApiFeedbackSent,
    DataExported,
    DeviceSignedOut,
    EmailChangeRequested,
    EmailChanged,
    EmailVerified,
    NewsletterSubscribed,
    NewsletterUnsubscribed,
    NotificationsUpdated,
    OtherDevicesSignedOut,
    PasskeyAdded,
    PasskeyPromptDeclined,
    PasskeyPromptSnoozed,
    PasskeyRemoved,
    PasskeyRenamed,
    PasswordChanged,
    PasswordReset,
    ProfileUpdated,
    RecoveryCodeUsed,
    RecoveryCodesRegenerated,
    TwoFactorDisabled,
    TwoFactorEnabled,
    SignedIn,
    AccountDisabled,
    AccountEnabled,
    AdminGranted,
    AdminRevoked,
    SignedOutEverywhere,
    TimeZoneChanged,
    VerificationResent,
    Billing(BillingEventKind),
}

impl AccountEventKind {
    pub const ALL: [Self; 40] = [
        Self::AccountCreated,
        Self::ApiTokenCreated,
        Self::ApiTokenRevoked,
        Self::ApiFeedbackSent,
        Self::DataExported,
        Self::DeviceSignedOut,
        Self::EmailChangeRequested,
        Self::EmailChanged,
        Self::EmailVerified,
        Self::NewsletterSubscribed,
        Self::NewsletterUnsubscribed,
        Self::NotificationsUpdated,
        Self::OtherDevicesSignedOut,
        Self::PasskeyAdded,
        Self::PasskeyPromptDeclined,
        Self::PasskeyPromptSnoozed,
        Self::PasskeyRemoved,
        Self::PasskeyRenamed,
        Self::PasswordChanged,
        Self::PasswordReset,
        Self::ProfileUpdated,
        Self::RecoveryCodeUsed,
        Self::RecoveryCodesRegenerated,
        Self::TwoFactorDisabled,
        Self::TwoFactorEnabled,
        Self::SignedIn,
        Self::AccountDisabled,
        Self::AccountEnabled,
        Self::AdminGranted,
        Self::AdminRevoked,
        Self::SignedOutEverywhere,
        Self::TimeZoneChanged,
        Self::VerificationResent,
        Self::Billing(BillingEventKind::CheckoutStarted),
        Self::Billing(BillingEventKind::PaymentFailed),
        Self::Billing(BillingEventKind::PlanChanged),
        Self::Billing(BillingEventKind::Subscribed),
        Self::Billing(BillingEventKind::SubscriptionCancelled),
        Self::Billing(BillingEventKind::SubscriptionEnded),
        Self::Billing(BillingEventKind::DuplicateRefunded),
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AccountCreated => "account_created",
            Self::ApiTokenCreated => "api_token_created",
            Self::ApiTokenRevoked => "api_token_revoked",
            Self::ApiFeedbackSent => "api_feedback_sent",
            Self::DataExported => "data_exported",
            Self::DeviceSignedOut => "device_signed_out",
            Self::EmailChangeRequested => "email_change_requested",
            Self::EmailChanged => "email_changed",
            Self::EmailVerified => "email_verified",
            Self::NewsletterSubscribed => "newsletter_subscribed",
            Self::NewsletterUnsubscribed => "newsletter_unsubscribed",
            Self::NotificationsUpdated => "notifications_updated",
            Self::OtherDevicesSignedOut => "other_devices_signed_out",
            Self::PasskeyAdded => "passkey_added",
            Self::PasskeyPromptDeclined => "passkey_prompt_declined",
            Self::PasskeyPromptSnoozed => "passkey_prompt_snoozed",
            Self::PasskeyRemoved => "passkey_removed",
            Self::PasskeyRenamed => "passkey_renamed",
            Self::PasswordChanged => "password_changed",
            Self::PasswordReset => "password_reset",
            Self::ProfileUpdated => "profile_updated",
            Self::RecoveryCodeUsed => "recovery_code_used",
            Self::RecoveryCodesRegenerated => "recovery_codes_regenerated",
            Self::TwoFactorDisabled => "two_factor_disabled",
            Self::TwoFactorEnabled => "two_factor_enabled",
            Self::SignedIn => "signed_in",
            Self::AccountDisabled => "account_disabled",
            Self::AccountEnabled => "account_enabled",
            Self::AdminGranted => "admin_granted",
            Self::AdminRevoked => "admin_revoked",
            Self::SignedOutEverywhere => "signed_out_everywhere",
            Self::TimeZoneChanged => "time_zone_changed",
            Self::VerificationResent => "verification_resent",
            Self::Billing(kind) => kind.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingEventKind {
    CheckoutStarted,
    PaymentFailed,
    PlanChanged,
    Subscribed,
    SubscriptionCancelled,
    SubscriptionEnded,
    DuplicateRefunded,
}

impl BillingEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CheckoutStarted => "checkout_started",
            Self::PaymentFailed => "payment_failed",
            Self::PlanChanged => "plan_changed",
            Self::Subscribed => "subscribed",
            Self::SubscriptionCancelled => "subscription_cancelled",
            Self::SubscriptionEnded => "subscription_ended",
            Self::DuplicateRefunded => "duplicate_subscription_refunded",
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
        let channels = [EventChannel::Browser, EventChannel::ApiToken(42)];
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
