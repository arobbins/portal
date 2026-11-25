use topcoat::router::StatusCode;

use super::sync::{Applied, Delivery, apply_change};
use crate::billing::catalog::find_price;
use crate::billing::jobs::is_stripe_id;
use crate::billing::queries::{query_get_subscription_plan, query_stripe_event_seen};
use crate::billing::webhook::{
    Change, EventKind, PlanRef, StripeEvent, SubscriptionPayload, parse_event, subscription,
    subscription_state,
};
use crate::env::Env;
use crate::stripe::{Request, Stripe, StripeError, verify_signature};

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
    let change = match resolve(env, stripe, &event).await? {
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

async fn resolve(
    env: &Env<'_>,
    stripe: &Stripe,
    event: &StripeEvent,
) -> topcoat::Result<Result<Change, StripeError>> {
    Ok(Ok(match &event.kind {
        EventKind::Subscription(payload) => {
            let current = match current_state(stripe, payload).await {
                Ok(current) => current,
                Err(error) => return Ok(Err(error)),
            };
            let plan = match plan_for(env, stripe, &current).await? {
                Ok(plan) => plan,
                Err(error) => return Ok(Err(error)),
            };
            plan.and_then(|plan| subscription_state(&current, &plan))
                .map_or(Change::Nothing, Change::Subscription)
        }
        EventKind::InvoicePaid { subscription } => Change::InvoicePaid {
            subscription: subscription.clone(),
        },
        EventKind::PaymentFailed { subscription } => Change::PaymentFailed {
            subscription: subscription.clone(),
        },
        EventKind::Other => Change::Nothing,
    }))
}

async fn current_state(
    stripe: &Stripe,
    payload: &SubscriptionPayload,
) -> Result<SubscriptionPayload, StripeError> {
    if !is_stripe_id(&payload.id, "sub_") {
        return Ok(payload.clone());
    }
    let path = format!("/v1/subscriptions/{}", payload.id);
    match stripe.send(Request::get(&path)).await {
        Ok(object) => Ok(subscription(&object, payload.deleted).unwrap_or_else(|| payload.clone())),
        Err(error) if error.is_not_found() => Ok(payload.clone()),
        Err(error) => Err(error),
    }
}

async fn plan_for(
    env: &Env<'_>,
    stripe: &Stripe,
    current: &SubscriptionPayload,
) -> topcoat::Result<Result<Option<PlanRef>, StripeError>> {
    let stored = query_get_subscription_plan(&current.id, env.db()).await?;
    if let Some(stored) = stored
        .as_ref()
        .filter(|plan| plan.price_id == current.price_id)
    {
        return Ok(Ok(Some(PlanRef {
            plan_key: stored.plan_key.clone(),
            plan_name: stored.plan_name.clone(),
            price_id: stored.price_id.clone(),
        })));
    }
    Ok(match find_price(stripe, &current.price_id).await {
        Ok(Some(price)) => Ok(Some(price.into())),
        Ok(None) => Ok(stored.map(PlanRef::from)),
        Err(error) => Err(error),
    })
}
