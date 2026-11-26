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
    LeftOut {
        table: "billing_customers",
        why: "Your Stripe customer id. Your billing details, invoices and receipts are kept by Stripe: open Manage billing to see or download them.",
    },
    LeftOut {
        table: "subscriptions",
        why: "Your plan and its renewal date, as Stripe last told us. Stripe's billing portal (Manage billing) has the full record.",
    },
    LeftOut {
        table: "checkout_requests",
        why: "Links to Stripe Checkout pages started for your account, kept for a day so a repeated request gets the same link.",
    },
    LeftOut {
        table: "oauth_requests",
        why: "Requests from apps waiting for you to approve them. They expire within minutes.",
    },
    LeftOut {
        table: "oauth_connections",
        why: "Apps you connected by signing in, with what each may do. The Connected apps card on the Developer tab lists them; their tokens are secrets and are never stored.",
    },
    LeftOut {
        table: "duplicate_subscriptions",
        why: "Stripe ids of any second subscription that was cancelled and refunded, so it's refunded only once. Your Recent activity lists each one.",
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
