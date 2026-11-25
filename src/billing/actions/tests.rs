mod checkout;
mod deletion;
mod duplicates;
mod follow_ups;
mod limits;
mod not_configured;
mod webhooks;

use std::sync::Arc;

use serde_json::{Value, json};
use sqlx::PgPool;

use super::webhook::WebhookReply;
use crate::action::Caller;
use crate::billing::queries::query_insert_customer;
use crate::billing::testing::WEBHOOK_SECRET;
use crate::env::App;
use crate::stripe::sign;
use crate::user::model::User;
use crate::user::testing::active_user;

pub const CUSTOMER: &str = "cus_fake1";

pub async fn customer(email: &str, customer_id: &str, db: &PgPool) -> Arc<User> {
    let user = active_user(email, db).await;
    query_insert_customer(user.id, customer_id, db)
        .await
        .unwrap();
    user
}

pub fn subscription(
    kind: &str,
    event: &str,
    created: i64,
    status: &str,
    cancelling: bool,
) -> Value {
    subscription_for(CUSTOMER, "sub_1", kind, event, created, status, cancelling)
}

pub fn subscription_for(
    customer: &str,
    sub: &str,
    kind: &str,
    event: &str,
    created: i64,
    status: &str,
    cancelling: bool,
) -> Value {
    json!({
        "id": event, "object": "event", "type": kind, "created": created,
        "data": { "object": {
            "id": sub, "object": "subscription", "customer": customer, "status": status,
            "cancel_at_period_end": cancelling, "cancel_at": if cancelling { json!(1_900_000_000) } else { Value::Null },
            "current_period_end": 1_900_000_000,
            "items": { "data": [{ "price": { "id": "price_pro", "product": "prod_pro" }, "current_period_end": 1_900_000_000 }] },
        } },
    })
}

pub fn invoice(kind: &str, event: &str, created: i64) -> Value {
    json!({
        "id": event, "object": "event", "type": kind, "created": created,
        "data": { "object": {
            "object": "invoice", "customer": CUSTOMER,
            "parent": { "subscription_details": { "subscription": "sub_1" } },
        } },
    })
}

pub async fn deliver(app: &App, event: &Value) -> WebhookReply {
    let body = event.to_string();
    let now = app.clock.now().unix_timestamp();
    let header = sign(WEBHOOK_SECRET, now, body.as_bytes());
    super::receive_stripe_event(&app.env(None), Some(&header), body.as_bytes())
        .await
        .unwrap()
}

pub async fn plan_of(app: &App, user: &Arc<User>) -> (String, super::BillingStatus) {
    let billing = super::get_billing(&app.env(None), &Caller::browser(user.clone()))
        .await
        .unwrap()
        .unwrap();
    (billing.plan, billing.status)
}

pub async fn activity_of(app: &App, user: &Arc<User>) -> Vec<String> {
    crate::activity::actions::activity_of(&app.env(None), user.id)
        .await
        .unwrap()
        .into_iter()
        .map(|item| item.summary)
        .collect()
}
