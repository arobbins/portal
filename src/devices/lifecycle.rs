use serde_json::json;
use sqlx::PgExecutor;
use time::Duration;

use super::queries::{
    NewDevice, query_delete_device, query_delete_stale_devices, query_delete_user_devices,
    query_get_signed_in_device, query_insert_device, query_move_device_to_version,
    query_touch_device,
};
use super::user_agent::{clip_user_agent, describe_user_agent};
use crate::env::{App, Env};
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::User;

pub const DEVICE_IDLE_LIMIT: Duration = Duration::days(1);
const LAST_SEEN_PRECISION: Duration = Duration::minutes(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInMethod {
    Password,
    Passkey,
    PasswordReset,
    NewAccount,
}

impl SignInMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Passkey => "passkey",
            Self::PasswordReset => "password_reset",
            Self::NewAccount => "new_account",
        }
    }
}

pub async fn start_device(
    env: &Env<'_>,
    user: &User,
    user_agent: &str,
    method: SignInMethod,
) -> topcoat::Result<String> {
    let public_id = PublicIdKind::Device.generate();
    let user_agent = clip_user_agent(user_agent);
    let device = NewDevice {
        public_id: &public_id,
        user_id: user.id,
        session_version: user.session_version,
        user_agent: &user_agent,
        ip: env.ip.map(|ip| ip.to_string()),
    };
    let detail = json!({
        "device": public_id,
        "name": describe_user_agent(&user_agent),
        "method": method.as_str(),
    });

    let mut tx = env.db().begin().await?;
    query_insert_device(&device, &mut *tx).await?;
    let source = EventSource::browser(user.id);
    record_event(env, &source, AccountEventKind::SignedIn, detail, &mut *tx).await?;
    tx.commit().await?;

    Ok(public_id)
}

pub async fn load_device(
    app: &App,
    user_id: i32,
    public_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    let Some(device) = query_get_signed_in_device(user_id, public_id, &app.db).await? else {
        return Ok(None);
    };
    let now = app.clock.now();
    if now - device.last_seen_at >= LAST_SEEN_PRECISION {
        query_touch_device(public_id, now, &app.db).await?;
    }

    Ok(Some(device.user))
}

pub async fn keep_device(app: &App, public_id: &str, user: &User) -> Result<bool, sqlx::Error> {
    query_move_device_to_version(public_id, user, &app.db).await
}

pub async fn end_device(app: &App, user_id: i32, public_id: &str) -> Result<(), sqlx::Error> {
    query_delete_device(user_id, public_id, &app.db).await?;
    Ok(())
}

pub async fn delete_stale_devices(app: &App) -> Result<u64, sqlx::Error> {
    query_delete_stale_devices(app.clock.now() - DEVICE_IDLE_LIMIT, &app.db).await
}

pub async fn end_every_device(user_id: i32, db: impl PgExecutor<'_>) -> Result<u64, sqlx::Error> {
    query_delete_user_devices(user_id, db).await
}
