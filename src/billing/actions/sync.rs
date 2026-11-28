use serde_json::json;
use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::billing::jobs::CancelStripeSubscription;
use crate::billing::queries::{
    query_get_current_subscription, query_insert_subscription, query_lock_customer,
    query_lock_subscription, query_record_duplicate_subscription, query_record_stripe_event,
    query_update_subscription, query_update_subscription_status,
};
use crate::billing::subscription::{Subscription, SubscriptionState, SubscriptionStatus};
use crate::billing::webhook::{Change, Snapshot, SubscriptionChange};
use crate::env::Env;
use crate::jobs::enqueue;
use crate::user::events::{AccountEventKind, BillingEventKind, EventSource, record_event};
use crate::user::queries::{query_get_user_by_id, query_get_user_by_public_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    Changed,
    Repeated,
    UnknownCustomer,
    CancelledForGoneAccount,
    UnknownSubscription,
    OutOfOrder,
    AlreadyEnded,
    Duplicate,
    Nothing,
}

pub struct Delivery<'a> {
    pub event_id: &'a str,
    pub created: OffsetDateTime,
    pub change: &'a Change,
}

pub async fn apply_change(env: &Env<'_>, delivery: &Delivery<'_>) -> topcoat::Result<Applied> {
    let mut tx = env.db().begin().await?;
    if !query_record_stripe_event(delivery.event_id, &mut *tx).await? {
        return Ok(Applied::Repeated);
    }
    let at = delivery.created;
    let applied = match delivery.change {
        Change::Subscription(change) => apply_subscription(env, change, at, &mut tx).await?,
        Change::PaymentFailed { subscription } => {
            apply_invoice(env, subscription, false, at, &mut tx).await?
        }
        Change::InvoicePaid { subscription } => {
            apply_invoice(env, subscription, true, at, &mut tx).await?
        }
        Change::Nothing => Applied::Nothing,
    };
    tx.commit().await?;
    Ok(applied)
}

async fn apply_subscription(
    env: &Env<'_>,
    change: &SubscriptionChange,
    at: OffsetDateTime,
    tx: &mut PgConnection,
) -> topcoat::Result<Applied> {
    let state = &change.state;
    let Some(user_id) = query_lock_customer(&state.customer_id, &mut *tx).await? else {
        return cancel_for_gone_account(change, tx).await;
    };
    let existing = query_lock_subscription(&state.stripe_subscription_id, &mut *tx).await?;
    let Some(before) = existing else {
        return start_subscription(env, user_id, state, at, tx).await;
    };
    if let Some(refused) = refusal(&before, at, change.snapshot) {
        return Ok(refused);
    }
    query_update_subscription(before.id, state, at, &mut *tx).await?;
    for kind in transitions(Some(&before), state) {
        log(env, user_id, kind, state, &mut *tx).await?;
    }
    Ok(Applied::Changed)
}

async fn start_subscription(
    env: &Env<'_>,
    user_id: i32,
    state: &SubscriptionState,
    at: OffsetDateTime,
    tx: &mut PgConnection,
) -> topcoat::Result<Applied> {
    if state.status == SubscriptionStatus::Ended {
        return Ok(Applied::UnknownSubscription);
    }
    if query_get_current_subscription(user_id, &mut *tx)
        .await?
        .is_some()
    {
        return refund_duplicate(env, user_id, state, tx).await;
    }
    query_insert_subscription(user_id, state, at, &mut *tx).await?;
    for kind in transitions(None, state) {
        log(env, user_id, kind, state, &mut *tx).await?;
    }
    Ok(Applied::Changed)
}

async fn refund_duplicate(
    env: &Env<'_>,
    user_id: i32,
    state: &SubscriptionState,
    tx: &mut PgConnection,
) -> topcoat::Result<Applied> {
    let subscription = &state.stripe_subscription_id;
    if !query_record_duplicate_subscription(subscription, user_id, &mut *tx).await? {
        return Ok(Applied::Duplicate);
    }
    tracing::warn!(user_id, subscription = %subscription, "second subscription for one account: cancelling and refunding it at Stripe");
    let job = CancelStripeSubscription {
        subscription: subscription.clone(),
        refund: true,
    };
    enqueue(&job, &mut *tx).await?;
    let kind = AccountEventKind::Billing(BillingEventKind::DuplicateRefunded);
    log(env, user_id, kind, state, &mut *tx).await?;
    Ok(Applied::Duplicate)
}

async fn cancel_for_gone_account(
    change: &SubscriptionChange,
    tx: &mut PgConnection,
) -> topcoat::Result<Applied> {
    let live = change.state.status != SubscriptionStatus::Ended;
    let Some(owner) = change.created_for.as_deref().filter(|_| live) else {
        return Ok(Applied::UnknownCustomer);
    };
    if query_get_user_by_public_id(owner, &mut *tx)
        .await?
        .is_some()
    {
        return Ok(Applied::UnknownCustomer);
    }
    let subscription = &change.state.stripe_subscription_id;
    tracing::warn!(subscription = %subscription, "subscription for a deleted account: cancelling and refunding it at Stripe");
    let job = CancelStripeSubscription {
        subscription: subscription.clone(),
        refund: true,
    };
    enqueue(&job, &mut *tx).await?;
    Ok(Applied::CancelledForGoneAccount)
}

fn refusal(before: &Subscription, at: OffsetDateTime, snapshot: Snapshot) -> Option<Applied> {
    if before.status == SubscriptionStatus::Ended {
        return Some(Applied::AlreadyEnded);
    }
    if snapshot == Snapshot::AsSent && at < before.last_event_at {
        return Some(Applied::OutOfOrder);
    }
    None
}

pub fn transitions(
    before: Option<&Subscription>,
    after: &SubscriptionState,
) -> Vec<AccountEventKind> {
    let Some(before) = before else {
        return vec![AccountEventKind::Billing(BillingEventKind::Subscribed)];
    };
    if after.status == SubscriptionStatus::Ended {
        return vec![AccountEventKind::Billing(
            BillingEventKind::SubscriptionEnded,
        )];
    }
    let mut kinds = Vec::new();
    if before.plan_key != after.plan_key {
        kinds.push(AccountEventKind::Billing(BillingEventKind::PlanChanged));
    }
    if before.ends_at.is_none() && after.ends_at.is_some() {
        kinds.push(AccountEventKind::Billing(
            BillingEventKind::SubscriptionCancelled,
        ));
    }
    kinds
}

async fn apply_invoice(
    env: &Env<'_>,
    subscription: &str,
    paid: bool,
    at: OffsetDateTime,
    tx: &mut PgConnection,
) -> topcoat::Result<Applied> {
    let Some(before) = query_lock_subscription(subscription, &mut *tx).await? else {
        return Ok(Applied::UnknownSubscription);
    };
    if let Some(refused) = refusal(&before, at, Snapshot::AsSent) {
        return Ok(refused);
    }
    let status = match (paid, before.status) {
        (true, SubscriptionStatus::PastDue) if before.ends_at.is_some() => {
            SubscriptionStatus::Cancelling
        }
        (true, SubscriptionStatus::PastDue) => SubscriptionStatus::Active,
        (true, status) => status,
        (false, _) => SubscriptionStatus::PastDue,
    };
    query_update_subscription_status(before.id, status, at, &mut *tx).await?;
    if !paid && before.status != SubscriptionStatus::PastDue {
        let state = state_of(&before);
        log(
            env,
            before.user_id,
            AccountEventKind::Billing(BillingEventKind::PaymentFailed),
            &state,
            &mut *tx,
        )
        .await?;
    }
    Ok(Applied::Changed)
}

fn state_of(row: &Subscription) -> SubscriptionState {
    SubscriptionState {
        stripe_subscription_id: row.stripe_subscription_id.clone(),
        customer_id: String::new(),
        plan_key: row.plan_key.clone(),
        plan_name: row.plan_name.clone(),
        price_id: row.price_id.clone(),
        status: row.status,
        current_period_end: row.current_period_end,
        ends_at: row.ends_at,
    }
}

async fn log(
    env: &Env<'_>,
    user_id: i32,
    kind: AccountEventKind,
    state: &SubscriptionState,
    tx: &mut PgConnection,
) -> topcoat::Result<()> {
    let mut detail = json!({
        "plan": state.plan_key,
        "plan_name": state.plan_name,
        "subscription": state.stripe_subscription_id,
    });
    if kind == AccountEventKind::Billing(BillingEventKind::SubscriptionCancelled)
        && let Some(ends_at) = state.ends_at
        && let Some(user) = query_get_user_by_id(user_id, &mut *tx).await?
    {
        detail["ends_on"] = json!(user.zone().date(ends_at));
    }
    record_event(env, &EventSource::stripe(user_id), kind, detail, &mut *tx).await
}
