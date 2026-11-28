use std::net::IpAddr;

use serde_json::json;
use sqlx::PgConnection;

use super::queries::{NewLogEntry, query_insert_admin_log};
use crate::user::events::{
    AccountEventKind, EventSource, NewAccountEvent, query_insert_account_event,
};
use crate::user::model::User;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum AdminLogAction {
    AccountDeleted,
    AccountDisabled,
    AccountEnabled,
    AdminGranted,
    AdminRevoked,
    SignedOutEverywhere,
    VerificationResent,
}

impl AdminLogAction {
    pub fn event_kind(self) -> Option<AccountEventKind> {
        match self {
            Self::AccountDeleted => None,
            Self::AccountDisabled => Some(AccountEventKind::AccountDisabled),
            Self::AccountEnabled => Some(AccountEventKind::AccountEnabled),
            Self::AdminGranted => Some(AccountEventKind::AdminGranted),
            Self::AdminRevoked => Some(AccountEventKind::AdminRevoked),
            Self::SignedOutEverywhere => Some(AccountEventKind::SignedOutEverywhere),
            Self::VerificationResent => Some(AccountEventKind::VerificationResent),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::AccountDeleted => "Deleted the account",
            Self::AccountDisabled => "Disabled the account",
            Self::AccountEnabled => "Enabled the account",
            Self::AdminGranted => "Made them an admin",
            Self::AdminRevoked => "Removed their admin role",
            Self::SignedOutEverywhere => "Signed them out everywhere",
            Self::VerificationResent => "Resent the confirmation email",
        }
    }
}

pub async fn log_admin_action(
    admin: Option<&User>,
    action: AdminLogAction,
    target: &User,
    ip: Option<IpAddr>,
    tx: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    let entry = NewLogEntry {
        admin_public_id: admin.map(|admin| admin.public_id.as_str()),
        admin_email: admin.map(|admin| admin.email.as_str()),
        action,
        target_public_id: &target.public_id,
        target_email: &target.email,
        ip: ip.map(|ip| ip.to_string()),
    };
    query_insert_admin_log(&entry, &mut *tx).await?;
    let Some(kind) = action.event_kind() else {
        return Ok(());
    };
    let (source, detail) = match admin {
        Some(admin) => (
            EventSource::admin(target.id),
            json!({ "admin": admin.public_id }),
        ),
        None => (EventSource::console(target.id), json!({})),
    };
    let event = NewAccountEvent {
        source: &source,
        kind,
        detail: &detail,
        ip,
    };
    query_insert_account_event(&event, &mut *tx).await
}
