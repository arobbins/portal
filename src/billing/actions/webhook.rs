use topcoat::router::StatusCode;

use super::sync::{Applied, Delivery, apply_change};
use crate::billing::catalog::find_price;
use crate::billing::queries::query_stripe_event_seen;
use crate::billing::webhook::{Change, EventKind, StripeEvent, parse_event, subscription_state};
use crate::env::Env;
use crate::stripe::{Stripe, StripeError, verify_signature};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookReply {
    Accepted(Applied),
    BadSignature,
    BadPayload,
    NotConfigured,
    StripeUnavailable,
}

impl WebhookReply {
    pub fn status(self) -> StatusCode {
        match self {
            Self::Accepted(_) => StatusCode::OK,
            Self::BadSignature | Self::BadPayload => StatusCode::BAD_REQUEST,
            Self::NotConfigured => StatusCode::NOT_FOUND,
            Self::StripeUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

pub async fn receive_stripe_event(
    env: &Env<'_>,
    signature: Option<&str>,
    payload: &[u8],
) -> topcoat::Result<WebhookReply> {
    let Some(stripe) = env.stripe() else {
        return Ok(WebhookReply::NotConfigured);
    };
    let now = env.now().unix_timestamp();
    if !verify_signature(stripe.webhook_secret(), signature, payload, now) {
        return Ok(WebhookReply::BadSignature);
    }
    let Some(event) = parse_event(payload) else {
        return Ok(WebhookReply::BadPayload);
    };
    if query_stripe_event_seen(&event.id, env.db()).await? {
        return Ok(WebhookReply::Accepted(Applied::Repeated));
    }
    let change = match resolve(stripe, &event).await {
        Ok(change) => change,
        Err(error) => {
            tracing::warn!(event = %event.id, "Stripe event not applied yet: {error}");
            return Ok(WebhookReply::StripeUnavailable);
        }
    };
    let delivery = Delivery {
        event_id: &event.id,
        created: event.created,
        change: &change,
    };
    let applied = apply_change(env, &delivery).await?;
    Ok(WebhookReply::Accepted(applied))
}

async fn resolve(stripe: &Stripe, event: &StripeEvent) -> Result<Change, StripeError> {
    Ok(match &event.kind {
        EventKind::Subscription(payload) => match find_price(stripe, &payload.price_id).await? {
            Some(price) => {
                subscription_state(payload, &price).map_or(Change::Nothing, Change::Subscription)
            }
            None => Change::Nothing,
        },
        EventKind::InvoicePaid { subscription } => Change::InvoicePaid {
            subscription: subscription.clone(),
        },
        EventKind::PaymentFailed { subscription } => Change::PaymentFailed {
            subscription: subscription.clone(),
        },
        EventKind::Other => Change::Nothing,
    })
}
