use crate::activity::export::activity_export;
use crate::api_tokens::export::api_tokens_export;
use crate::devices::export::devices_export;
use crate::env::Env;
use crate::feedback::export::api_feedback_export;
use crate::passkeys::export::passkeys_export;
use crate::preferences::export::notifications_export;
use crate::profile::export::profile_export;
use crate::two_factor::export::two_factor_export;
use crate::user::model::User;

use super::dataset::Dataset;

pub struct LeftOut {
    pub table: &'static str,
    pub why: &'static str,
}

pub const LEFT_OUT: &[LeftOut] = &[
    LeftOut {
        table: "known_devices",
        why: "Browsers you've signed in from before, used for new sign-in alerts. Only a fingerprint of a random cookie is stored, which means nothing outside this app.",
    },
    LeftOut {
        table: "user_tokens",
        why: "Links we emailed you (confirmations and password resets). They're secrets and expire quickly.",
    },
    LeftOut {
        table: "recovery_codes",
        why: "Two-factor recovery codes. They're secrets; we only keep a fingerprint of each.",
    },
];

pub async fn collect(env: &Env<'_>, user: &User) -> Result<Vec<Dataset>, sqlx::Error> {
    Ok(vec![
        profile_export(env, user).await?,
        passkeys_export(env, user).await?,
        api_tokens_export(env, user).await?,
        devices_export(env, user).await?,
        activity_export(env, user).await?,
        notifications_export(env, user).await?,
        two_factor_export(env, user).await?,
        api_feedback_export(env, user).await?,
    ])
}

#[cfg(test)]
mod tests;
