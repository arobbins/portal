use topcoat::router::StatusCode;

use super::stripe_call::{StripeCall, split};
use super::sync::{Applied, Delivery, apply_change};
use crate::billing::catalog::find_price;
use crate::billing::queries::{query_get_subscription_plan, query_stripe_event_seen};
use crate::billing::webhook::{
    Change, EventKind, PlanRef, Snapshot, StripeEvent, SubscriptionChange, SubscriptionPayload,
    parse_event, subscription, subscription_state,
};
use crate::env::Env;
use crate::secret::PublicIdKind;
use crate::stripe::is_id;
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
    let change = match split(resolve(env, stripe, &event).await)? {
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

async fn resolve(env: &Env<'_>, stripe: &Stripe, event: &StripeEvent) -> StripeCall<Change> {
    Ok(match &event.kind {
        EventKind::Subscription(payload) => {
            let (current, snapshot) = current_state(stripe, payload).await?;
            let plan = plan_for(env, stripe, &current).await?;
            let created_for = created_for(env, &current, snapshot);
            plan.and_then(|plan| subscription_state(&current, &plan))
                .map_or(Change::Nothing, |state| {
                    Change::Subscription(SubscriptionChange {
                        state,
                        snapshot,
                        created_for,
                    })
                })
        }
        EventKind::InvoicePaid { subscription } => Change::InvoicePaid {
            subscription: subscription.clone(),
        },
        EventKind::PaymentFailed { subscription } => Change::PaymentFailed {
            subscription: subscription.clone(),
        },
        EventKind::Other => Change::Nothing,
    })
}

async fn current_state(
    stripe: &Stripe,
    payload: &SubscriptionPayload,
) -> Result<(SubscriptionPayload, Snapshot), StripeError> {
    let as_sent = (payload.clone(), Snapshot::AsSent);
    if !is_id(&payload.id, "sub_") {
        return Ok(as_sent);
    }
    let path = format!("/v1/subscriptions/{}?expand[]=customer", payload.id);
    match stripe.send(Request::get(&path)).await {
        Ok(object) => Ok(subscription(&object, payload.deleted)
            .map(|current| (current, Snapshot::Current))
            .unwrap_or(as_sent)),
        Err(error) if error.is_not_found() => Ok(as_sent),
        Err(error) => Err(error),
    }
}

fn created_for(env: &Env<'_>, current: &SubscriptionPayload, snapshot: Snapshot) -> Option<String> {
    let owner = current.owner.as_ref()?;
    let ours = snapshot == Snapshot::Current
        && owner.app == env.app.config.origin()
        && PublicIdKind::User.is_valid(&owner.user);
    ours.then(|| owner.user.clone())
}

async fn plan_for(
    env: &Env<'_>,
    stripe: &Stripe,
    current: &SubscriptionPayload,
) -> StripeCall<Option<PlanRef>> {
    let stored = match query_get_subscription_plan(&current.id, env.db()).await? {
        Some(plan) if plan.price_id == current.price_id => return Ok(Some(plan.into())),
        stored => stored,
    };
    Ok(match find_price(stripe, &current.price_id).await? {
        Some(price) => Some(price.into()),
        None => stored.map(PlanRef::from),
    })
}
