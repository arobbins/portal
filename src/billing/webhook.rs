use crate::stripe::expandable_id;
use serde_json::Value;
use time::OffsetDateTime;

use super::catalog::PaidPrice;
use super::queries::StoredPlan;
use super::subscription::{SubscriptionState, SubscriptionStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripeEvent {
    pub id: String,
    pub created: OffsetDateTime,
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    Subscription(SubscriptionPayload),
    InvoicePaid { subscription: String },
    PaymentFailed { subscription: String },
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripeStatus {
    Live,
    Unpaid,
    Over,
    Other,
}

impl StripeStatus {
    fn parse(value: &str) -> Self {
        match value {
            "active" | "trialing" => Self::Live,
            "past_due" | "unpaid" => Self::Unpaid,
            "canceled" | "incomplete_expired" => Self::Over,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionPayload {
    pub id: String,
    pub customer: String,
    pub price_id: String,
    pub stripe_status: StripeStatus,
    pub deleted: bool,
    pub cancel_at_period_end: bool,
    pub cancel_at: Option<i64>,
    pub current_period_end: i64,
    pub owner: Option<CustomerTag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerTag {
    pub user: String,
    pub app: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Snapshot {
    Current,
    AsSent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionChange {
    pub state: SubscriptionState,
    pub snapshot: Snapshot,
    pub created_for: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Subscription(SubscriptionChange),
    InvoicePaid { subscription: String },
    PaymentFailed { subscription: String },
    Nothing,
}

pub fn parse_event(payload: &[u8]) -> Option<StripeEvent> {
    let event: Value = serde_json::from_slice(payload).ok()?;
    let id = event["id"]
        .as_str()
        .filter(|id| id.len() <= 255)?
        .to_string();
    let created = OffsetDateTime::from_unix_timestamp(event["created"].as_i64()?).ok()?;
    let object = &event["data"]["object"];
    let kind = match event["type"].as_str()? {
        "customer.subscription.created" | "customer.subscription.updated" => {
            subscription(object, false).map_or(EventKind::Other, EventKind::Subscription)
        }
        "customer.subscription.deleted" => {
            subscription(object, true).map_or(EventKind::Other, EventKind::Subscription)
        }
        "invoice.paid" => invoice_subscription(object).map_or(EventKind::Other, |subscription| {
            EventKind::InvoicePaid { subscription }
        }),
        "invoice.payment_failed" => invoice_subscription(object)
            .map_or(EventKind::Other, |subscription| EventKind::PaymentFailed {
                subscription,
            }),
        _ => EventKind::Other,
    };
    Some(StripeEvent { id, created, kind })
}

fn id_of(value: &Value) -> Option<String> {
    expandable_id(value).map(str::to_string)
}

pub fn subscription(object: &Value, deleted: bool) -> Option<SubscriptionPayload> {
    let item = &object["items"]["data"][0];
    Some(SubscriptionPayload {
        id: object["id"].as_str()?.to_string(),
        customer: id_of(&object["customer"])?,
        price_id: id_of(&item["price"])?,
        stripe_status: StripeStatus::parse(object["status"].as_str()?),
        deleted,
        cancel_at_period_end: object["cancel_at_period_end"].as_bool().unwrap_or(false),
        cancel_at: object["cancel_at"].as_i64(),
        current_period_end: object["current_period_end"]
            .as_i64()
            .or_else(|| item["current_period_end"].as_i64())?,
        owner: customer_tag(&object["customer"]),
    })
}

fn customer_tag(customer: &Value) -> Option<CustomerTag> {
    let metadata = &customer["metadata"];
    Some(CustomerTag {
        user: metadata["user"].as_str()?.to_string(),
        app: metadata["app"].as_str()?.to_string(),
    })
}

fn invoice_subscription(object: &Value) -> Option<String> {
    id_of(&object["subscription"])
        .or_else(|| id_of(&object["parent"]["subscription_details"]["subscription"]))
}

fn status_of(payload: &SubscriptionPayload) -> Option<SubscriptionStatus> {
    if payload.deleted {
        return Some(SubscriptionStatus::Ended);
    }
    match payload.stripe_status {
        StripeStatus::Live if payload.cancel_at_period_end || payload.cancel_at.is_some() => {
            Some(SubscriptionStatus::Cancelling)
        }
        StripeStatus::Live => Some(SubscriptionStatus::Active),
        StripeStatus::Unpaid => Some(SubscriptionStatus::PastDue),
        StripeStatus::Over => Some(SubscriptionStatus::Ended),
        StripeStatus::Other => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRef {
    pub plan_key: String,
    pub plan_name: String,
    pub price_id: String,
}

impl From<PaidPrice> for PlanRef {
    fn from(price: PaidPrice) -> Self {
        Self {
            plan_key: price.plan_key,
            plan_name: price.plan_name,
            price_id: price.price_id,
        }
    }
}

impl From<StoredPlan> for PlanRef {
    fn from(plan: StoredPlan) -> Self {
        Self {
            plan_key: plan.plan_key,
            plan_name: plan.plan_name,
            price_id: plan.price_id,
        }
    }
}

pub fn subscription_state(
    payload: &SubscriptionPayload,
    price: &PlanRef,
) -> Option<SubscriptionState> {
    let status = status_of(payload)?;
    let current_period_end =
        OffsetDateTime::from_unix_timestamp(payload.current_period_end).ok()?;
    let scheduled = payload.cancel_at_period_end || payload.cancel_at.is_some();
    let ends_at = (scheduled && status != SubscriptionStatus::Ended).then(|| {
        payload
            .cancel_at
            .and_then(|at| OffsetDateTime::from_unix_timestamp(at).ok())
            .unwrap_or(current_period_end)
    });
    Some(SubscriptionState {
        stripe_subscription_id: payload.id.clone(),
        customer_id: payload.customer.clone(),
        plan_key: price.plan_key.clone(),
        plan_name: price.plan_name.clone(),
        price_id: price.price_id.clone(),
        status,
        current_period_end,
        ends_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(kind: &str, object: Value) -> Vec<u8> {
        json!({ "id": "evt_1", "type": kind, "created": 1_800_000_000, "data": { "object": object } })
            .to_string()
            .into_bytes()
    }

    fn sub(status: &str) -> Value {
        json!({
            "id": "sub_1", "customer": "cus_1", "status": status,
            "cancel_at_period_end": false, "cancel_at": null,
            "items": { "data": [{ "price": { "id": "price_pro" }, "current_period_end": 1_802_000_000 }] },
        })
    }

    #[test]
    fn subscription_events_carry_the_price_and_period() {
        let parsed = parse_event(&event("customer.subscription.created", sub("active"))).unwrap();

        let EventKind::Subscription(payload) = parsed.kind else {
            panic!("not a subscription event");
        };
        assert_eq!(payload.price_id, "price_pro");
        assert_eq!(payload.current_period_end, 1_802_000_000);
        assert_eq!(status_of(&payload), Some(SubscriptionStatus::Active));
    }

    #[test]
    fn stripe_statuses_map_to_ours() {
        let payload = |status: &str, cancelling: bool, deleted: bool| SubscriptionPayload {
            id: "sub_1".into(),
            customer: "cus_1".into(),
            price_id: "price_pro".into(),
            stripe_status: StripeStatus::parse(status),
            deleted,
            cancel_at_period_end: cancelling,
            cancel_at: None,
            current_period_end: 0,
            owner: None,
        };

        assert_eq!(
            status_of(&payload("active", true, false)),
            Some(SubscriptionStatus::Cancelling)
        );
        assert_eq!(
            status_of(&payload("past_due", false, false)),
            Some(SubscriptionStatus::PastDue)
        );
        assert_eq!(
            status_of(&payload("active", false, true)),
            Some(SubscriptionStatus::Ended)
        );
        assert_eq!(status_of(&payload("incomplete", false, false)), None);
    }

    #[test]
    fn invoices_name_their_subscription_either_way() {
        let old = json!({ "subscription": "sub_1" });
        let new = json!({ "parent": { "subscription_details": { "subscription": "sub_2" } } });

        assert_eq!(
            parse_event(&event("invoice.payment_failed", old))
                .unwrap()
                .kind,
            EventKind::PaymentFailed {
                subscription: "sub_1".into()
            }
        );
        assert_eq!(
            parse_event(&event("invoice.paid", new)).unwrap().kind,
            EventKind::InvoicePaid {
                subscription: "sub_2".into()
            }
        );
    }

    #[test]
    fn other_events_and_junk_are_told_apart() {
        assert_eq!(
            parse_event(&event("checkout.session.completed", json!({})))
                .unwrap()
                .kind,
            EventKind::Other
        );
        assert!(parse_event(b"not json").is_none());
    }
}
