pub mod actions;
pub mod api;
pub mod catalog;
pub mod export;
pub mod jobs;
pub mod pages;
mod period;
mod queries;
mod subscription;
#[cfg(test)]
pub mod testing;
mod webhook;

use std::sync::Arc;

use sqlx::PgConnection;

use crate::action::{Caller, ForPage};
use crate::env::{App, Env};
use crate::jobs::enqueue;
use crate::user::queries::query_get_user_by_id;
use actions::{BillingStatus, StartCheckout, get_billing, start_checkout};
use jobs::{CancelStripeSubscription, UpdateStripeCustomerEmail};
use queries::{
    query_delete_old_checkout_requests, query_delete_old_stripe_events,
    query_get_current_subscription, query_get_customer_id, query_lock_customer_of_public_user,
    query_lock_customer_of_user,
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

pub async fn lock_before_deleting(
    public_id: &str,
    tx: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    query_lock_customer_of_public_user(public_id, &mut *tx).await?;
    Ok(())
}

pub async fn cancel_before_deleting(
    user_id: i32,
    tx: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    query_lock_customer_of_user(user_id, &mut *tx).await?;
    let Some(subscription) = query_get_current_subscription(user_id, &mut *tx).await? else {
        return Ok(());
    };
    let job = CancelStripeSubscription {
        subscription: subscription.stripe_subscription_id,
        refund: false,
    };
    enqueue(&job, &mut *tx).await
}

pub async fn follow_email_change(user_id: i32, tx: &mut PgConnection) -> Result<(), sqlx::Error> {
    let Some(customer) = query_get_customer_id(user_id, &mut *tx).await? else {
        return Ok(());
    };
    let job = UpdateStripeCustomerEmail { customer };
    enqueue(&job, &mut *tx).await
}

pub async fn subscription_to_cancel(
    env: &Env<'_>,
    caller: &Caller,
) -> topcoat::Result<Option<String>> {
    let billing = get_billing(env, caller).await?.for_page()?;
    Ok((billing.status != BillingStatus::Free).then_some(billing.plan_name))
}

pub async fn checkout_for_price(
    env: &Env<'_>,
    user_id: i32,
    price: String,
) -> topcoat::Result<Option<String>> {
    let Some(user) = query_get_user_by_id(user_id, env.db()).await? else {
        return Ok(None);
    };
    let caller = Caller::browser(Arc::new(user));
    match start_checkout(env, &caller, &StartCheckout { price }).await? {
        Ok(link) => Ok(Some(link.checkout_url)),
        Err(error) => {
            tracing::info!("no checkout after sign-in: {:?}", error.for_browser()?);
            Ok(None)
        }
    }
}
