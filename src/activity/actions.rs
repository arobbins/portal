use std::convert::Infallible;

use serde_json::Value;
use time::OffsetDateTime;

use crate::action::{ActionResult, Caller, authorize};
use crate::env::Env;
use crate::permission::Permission;
use crate::user::event_history::{PastEvent, query_list_recent_account_events};
use crate::user::events::{APP_DETAIL, AccountEventKind, BillingEventKind, EventChannel};

pub const RECENT_ACTIVITY_LIMIT: i64 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityView {
    pub summary: String,
    pub device: Option<String>,
    pub ip: Option<String>,
    pub by: Option<String>,
    pub at: OffsetDateTime,
}

pub async fn list_recent_activity(
    env: &Env<'_>,
    caller: &Caller,
) -> ActionResult<Vec<ActivityView>, Infallible> {
    authorize!(caller.require(Permission::ViewActivity));

    Ok(Ok(activity_of(env, caller.owner_id()).await?))
}

pub async fn activity_of(env: &Env<'_>, user_id: i32) -> Result<Vec<ActivityView>, sqlx::Error> {
    let events = query_list_recent_account_events(user_id, RECENT_ACTIVITY_LIMIT, env.db()).await?;

    Ok(events.into_iter().map(ActivityView::of).collect())
}

pub async fn activity_history(
    env: &Env<'_>,
    user_id: i32,
) -> Result<Vec<ActivityView>, sqlx::Error> {
    let events = query_list_recent_account_events(user_id, i64::MAX, env.db()).await?;

    Ok(events.into_iter().map(ActivityView::of).collect())
}

impl ActivityView {
    fn of(event: PastEvent) -> Self {
        let device = match event.kind {
            AccountEventKind::SignedIn => text(&event.detail, "name"),
            _ => None,
        };

        Self {
            summary: summarize(event.kind, &event.detail),
            device,
            ip: event.ip,
            by: actor(event.via, &event.detail),
            at: event.at,
        }
    }
}

fn summarize(kind: AccountEventKind, detail: &Value) -> String {
    let name = text(detail, "name").unwrap_or_default();
    match kind {
        AccountEventKind::SignedIn => signed_in(detail),
        AccountEventKind::OtherDevicesSignedOut => match detail["devices"].as_u64() {
            Some(1) => "1 other device signed out".to_string(),
            Some(count) => format!("{count} other devices signed out"),
            None => "Other devices signed out".to_string(),
        },
        AccountEventKind::DeviceSignedOut => format!("{name} signed out"),
        AccountEventKind::ApiTokenCreated => format!("API token “{name}” created"),
        AccountEventKind::ApiTokenRevoked => format!("API token “{name}” revoked"),
        AccountEventKind::AppConnected => format!("Connected {name}"),
        AccountEventKind::AppDisconnected => format!("Disconnected {name}"),
        AccountEventKind::PasskeyAdded => format!("Passkey “{name}” added"),
        AccountEventKind::PasskeyRenamed => format!("Passkey renamed to “{name}”"),
        AccountEventKind::PasskeyRemoved => format!("Passkey “{name}” removed"),
        AccountEventKind::NewsletterSubscribed => format!("Subscribed to {name}"),
        AccountEventKind::NewsletterUnsubscribed => format!("Unsubscribed from {name}"),
        AccountEventKind::Billing(kind) => billing_summary(kind, detail),
        other => fixed_summary(other).to_string(),
    }
}

fn actor(channel: EventChannel, detail: &Value) -> Option<String> {
    let fixed = match channel {
        EventChannel::Browser | EventChannel::Stripe => None,
        EventChannel::ApiToken(_) => Some("API token"),
        EventChannel::App => return text(detail, APP_DETAIL).map(|app| format!("By {app}")),
        EventChannel::Admin => Some("By an admin"),
        EventChannel::Console => Some("By the server"),
    };
    fixed.map(str::to_string)
}

fn billing_summary(kind: BillingEventKind, detail: &Value) -> String {
    let plan = text(detail, "plan_name").unwrap_or_else(|| "a plan".to_string());
    match kind {
        BillingEventKind::Subscribed => format!("Subscribed to {plan}"),
        BillingEventKind::PlanChanged => format!("Changed plan to {plan}"),
        BillingEventKind::SubscriptionCancelled => match text(detail, "ends_on") {
            Some(ends_on) => format!("Cancelled {plan} (ends {ends_on})"),
            None => format!("Cancelled {plan}"),
        },
        BillingEventKind::PaymentFailed => format!("Payment failed for {plan}"),
        BillingEventKind::SubscriptionEnded => format!("{plan} ended, back on Free"),
        BillingEventKind::DuplicateRefunded => {
            format!("Second {plan} subscription cancelled and refunded")
        }
        BillingEventKind::CheckoutStarted => {
            match (text(detail, "token"), text(detail, APP_DETAIL)) {
                (Some(token), _) => format!("Checkout for {plan} started by token {token}"),
                (None, Some(app)) => format!("Checkout for {plan} started by {app}"),
                (None, None) => format!("Checkout for {plan} started"),
            }
        }
    }
}

fn fixed_summary(kind: AccountEventKind) -> &'static str {
    match kind {
        AccountEventKind::AccountCreated => "Account created",
        AccountEventKind::EmailVerified => "Email confirmed",
        AccountEventKind::EmailChangeRequested => "Email change requested",
        AccountEventKind::EmailChanged => "Email changed",
        AccountEventKind::PasswordChanged => "Password changed",
        AccountEventKind::PasswordReset => "Password reset",
        AccountEventKind::ProfileUpdated => "Profile updated",
        AccountEventKind::ApiFeedbackSent => "API feedback sent",
        AccountEventKind::DataExported => "Account data exported",
        AccountEventKind::RecoveryCodeUsed => "Signed in with a recovery code",
        AccountEventKind::RecoveryCodesRegenerated => "New recovery codes created",
        AccountEventKind::TwoFactorDisabled => "Two-factor authentication turned off",
        AccountEventKind::TwoFactorEnabled => "Two-factor authentication turned on",
        AccountEventKind::PasskeyPromptSnoozed => "Passkey suggestion postponed",
        AccountEventKind::PasskeyPromptDeclined => "Passkey suggestions turned off",
        AccountEventKind::AccountDisabled => "Account disabled",
        AccountEventKind::AccountEnabled => "Account enabled",
        AccountEventKind::AdminGranted => "Admin role granted",
        AccountEventKind::AdminRevoked => "Admin role removed",
        AccountEventKind::SignedOutEverywhere => "Signed out of every device",
        AccountEventKind::TimeZoneChanged => "Time zone changed",
        AccountEventKind::NotificationsUpdated => "Notification settings changed",
        AccountEventKind::VerificationResent => "Confirmation email resent",
        _ => "Account changed",
    }
}

fn signed_in(detail: &Value) -> String {
    let how = match detail["method"].as_str() {
        Some("passkey") => "with a passkey",
        Some("password_reset") => "after a password reset",
        Some("new_account") => "after setting a password",
        _ => "with a password",
    };
    format!("Signed in {how}")
}

fn text(detail: &Value, key: &str) -> Option<String> {
    detail[key].as_str().map(str::to_string)
}

#[cfg(test)]
mod tests;
