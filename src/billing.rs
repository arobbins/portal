pub mod actions;
pub mod api;
pub mod catalog;
pub mod jobs;
pub mod pages;
mod queries;
pub mod subscription;
#[cfg(test)]
pub mod testing;
pub mod webhook;

use sqlx::PgConnection;

use crate::env::App;
use crate::jobs::enqueue;
use jobs::{CancelStripeSubscription, UpdateStripeCustomerEmail};
use queries::{
    query_delete_old_checkout_requests, query_delete_old_stripe_events,
    query_get_current_subscription, query_get_customer_id,
};

const STRIPE_EVENT_RETENTION: time::Duration = time::Duration::days(30);
const CHECKOUT_REQUEST_RETENTION: time::Duration = time::Duration::days(1);

pub async fn delete_expired(app: &App) -> Result<(u64, u64), sqlx::Error> {
    let now = app.clock.now();
    let events = query_delete_old_stripe_events(now - STRIPE_EVENT_RETENTION, &app.db).await?;
    let requests =
        query_delete_old_checkout_requests(now - CHECKOUT_REQUEST_RETENTION, &app.db).await?;
    Ok((events, requests))
}

pub async fn cancel_before_deleting(
    user_id: i32,
    tx: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    let Some(subscription) = query_get_current_subscription(user_id, &mut *tx).await? else {
        return Ok(());
    };
    let job = CancelStripeSubscription {
        subscription: subscription.stripe_subscription_id,
        refund: false,
    };
    enqueue(&job, &mut *tx).await
}

pub async fn follow_email_change(
    user_id: i32,
    email: &str,
    tx: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    let Some(customer) = query_get_customer_id(user_id, &mut *tx).await? else {
        return Ok(());
    };
    let job = UpdateStripeCustomerEmail {
        customer,
        email: email.to_string(),
    };
    enqueue(&job, &mut *tx).await
}
