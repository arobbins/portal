use sqlx::PgExecutor;
use time::OffsetDateTime;

use super::subscription::{Subscription, SubscriptionState, SubscriptionStatus};

pub async fn query_get_customer_id(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT stripe_customer_id FROM billing_customers WHERE user_id = $1",
        user_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_insert_customer(
    user_id: i32,
    customer_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO billing_customers (user_id, stripe_customer_id) VALUES ($1, $2) ON CONFLICT (user_id) DO NOTHING",
        user_id,
        customer_id,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_lock_customer(
    customer_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT user_id FROM billing_customers WHERE stripe_customer_id = $1 FOR UPDATE",
        customer_id,
    )
    .fetch_optional(db)
    .await
}

pub struct StoredPlan {
    pub plan_key: String,
    pub plan_name: String,
    pub price_id: String,
}

pub async fn query_get_subscription_plan(
    stripe_subscription_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<StoredPlan>, sqlx::Error> {
    sqlx::query_as!(
        StoredPlan,
        "SELECT plan_key, plan_name, price_id FROM subscriptions WHERE stripe_subscription_id = $1",
        stripe_subscription_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_get_current_subscription(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as!(
        Subscription,
        r#"SELECT id, user_id, stripe_subscription_id, plan_key, plan_name, price_id,
            status AS "status: SubscriptionStatus", current_period_end, ends_at, last_event_at
        FROM subscriptions WHERE user_id = $1 AND status <> 'ended'
        ORDER BY created_at DESC, id DESC LIMIT 1"#,
        user_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_has_any_subscription(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let found = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM subscriptions WHERE user_id = $1) AS "found!""#,
        user_id,
    )
    .fetch_one(db)
    .await?;

    Ok(found)
}

pub async fn query_lock_subscription(
    stripe_subscription_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as!(
        Subscription,
        r#"SELECT id, user_id, stripe_subscription_id, plan_key, plan_name, price_id,
            status AS "status: SubscriptionStatus", current_period_end, ends_at, last_event_at
        FROM subscriptions WHERE stripe_subscription_id = $1 FOR UPDATE"#,
        stripe_subscription_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_insert_subscription(
    user_id: i32,
    state: &SubscriptionState,
    at: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO subscriptions (user_id, stripe_subscription_id, plan_key, plan_name, price_id, status, current_period_end, ends_at, last_event_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        user_id,
        state.stripe_subscription_id,
        state.plan_key,
        state.plan_name,
        state.price_id,
        state.status.as_str(),
        state.current_period_end,
        state.ends_at,
        at,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_update_subscription(
    id: i32,
    state: &SubscriptionState,
    at: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET plan_key = $2, plan_name = $3, price_id = $4, status = $5,
            current_period_end = $6, ends_at = $7, last_event_at = $8, updated_at = now()
        WHERE id = $1",
        id,
        state.plan_key,
        state.plan_name,
        state.price_id,
        state.status.as_str(),
        state.current_period_end,
        state.ends_at,
        at,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_update_subscription_status(
    id: i32,
    status: SubscriptionStatus,
    at: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = $2, last_event_at = $3, updated_at = now() WHERE id = $1",
        id,
        status.as_str(),
        at,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_stripe_event_seen(
    event_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let seen = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM stripe_events WHERE event_id = $1) AS "seen!""#,
        event_id,
    )
    .fetch_one(db)
    .await?;

    Ok(seen)
}

pub async fn query_record_stripe_event(
    event_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "INSERT INTO stripe_events (event_id) VALUES ($1) ON CONFLICT DO NOTHING",
        event_id,
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn query_delete_old_stripe_events(
    cutoff: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM stripe_events WHERE received_at < $1", cutoff)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

pub struct CheckoutRequest {
    pub price_id: String,
    pub checkout_url: String,
}

pub async fn query_get_checkout_request(
    user_id: i32,
    idempotency_key: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<CheckoutRequest>, sqlx::Error> {
    sqlx::query_as!(
        CheckoutRequest,
        "SELECT price_id, checkout_url FROM checkout_requests WHERE user_id = $1 AND idempotency_key = $2",
        user_id,
        idempotency_key,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_insert_checkout_request(
    user_id: i32,
    idempotency_key: &str,
    request: &CheckoutRequest,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO checkout_requests (user_id, idempotency_key, price_id, checkout_url) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        user_id,
        idempotency_key,
        request.price_id,
        request.checkout_url,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_delete_old_checkout_requests(
    cutoff: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM checkout_requests WHERE created_at < $1",
        cutoff
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}
