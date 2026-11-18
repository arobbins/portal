use axum::http::HeaderMap;
use axum::http::header::COOKIE;
use time::Duration;

use super::emails::{SignInNotice, new_sign_in_email};
use super::lifecycle::SignInMethod;
use super::queries::{
    query_delete_stale_known_devices, query_insert_known_device, query_touch_known_device,
};
use super::user_agent::describe_user_agent;
use crate::account::tabs::AccountSection;
use crate::env::{App, Env};
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::preferences::notifications::{NotificationTopic, wants};
use crate::secret::{hash_secret, random_base62};
use crate::user::model::User;

pub const KNOWN_DEVICE_COOKIE: &str = "known_device";
pub const KNOWN_DEVICE_LIFETIME: Duration = Duration::days(400);
const TOKEN_LENGTH: usize = 32;

pub fn known_device_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == KNOWN_DEVICE_COOKIE)
        .map(|(_, value)| value.to_string())
        .filter(|token| is_well_formed(token))
}

pub fn known_device_cookie(token: &str, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!(
        "{KNOWN_DEVICE_COOKIE}={token}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{secure}",
        KNOWN_DEVICE_LIFETIME.whole_seconds()
    )
}

fn is_well_formed(token: &str) -> bool {
    token.len() == TOKEN_LENGTH && token.chars().all(|c| c.is_ascii_alphanumeric())
}

fn alerts_for(method: SignInMethod) -> bool {
    match method {
        SignInMethod::Password | SignInMethod::Passkey => true,
        SignInMethod::PasswordReset | SignInMethod::NewAccount => false,
    }
}

pub async fn recognize_device(
    env: &Env<'_>,
    user: &User,
    token: Option<&str>,
    method: SignInMethod,
    user_agent: &str,
) -> topcoat::Result<String> {
    let token = token
        .filter(|token| is_well_formed(token))
        .map_or_else(|| random_base62(TOKEN_LENGTH), str::to_string);
    let hash = hash_secret(&token);
    if query_touch_known_device(user.id, &hash, env.now(), env.db()).await? {
        return Ok(token);
    }

    let mut tx = env.db().begin().await?;
    query_insert_known_device(user.id, &hash, &mut *tx).await?;
    if alerts_for(method)
        && wants(user.id, NotificationTopic::SignInAlerts, &mut *tx).await?
        && env.limits().allow_email_to(&user.email).await?
    {
        let ip = env.ip.map(|ip| ip.to_string());
        let notice = SignInNotice {
            device: &describe_user_agent(user_agent),
            ip: ip.as_deref(),
            when: &user.zone().date_time(env.now()),
            devices_url: &env.app.url(&AccountSection::Devices.path()),
            settings_url: &env.app.url(&AccountSection::Notifications.path()),
        };
        enqueue(
            &SendEmail(new_sign_in_email(&user.email, &notice)),
            &mut *tx,
        )
        .await?;
    }
    tx.commit().await?;

    Ok(token)
}

pub async fn forget_unseen_devices(app: &App) -> Result<u64, sqlx::Error> {
    query_delete_stale_known_devices(app.clock.now() - KNOWN_DEVICE_LIFETIME, &app.db).await
}

#[cfg(test)]
mod tests;
