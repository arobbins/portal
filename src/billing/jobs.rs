use serde::{Deserialize, Serialize};

use super::queries::query_get_user_of_customer;
use crate::env::App;
use crate::jobs::{Job, JobError};
use crate::stripe::{Request, Stripe, expandable_id, is_id};
use crate::user::queries::query_get_user_by_id;

const RETRY_FOR_DAYS_OF_ATTEMPTS: i32 = 100;
const ALREADY_REFUNDED: &str = "charge_already_refunded";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelStripeSubscription {
    pub subscription: String,
    #[serde(default)]
    pub refund: bool,
}

impl Job for CancelStripeSubscription {
    const KIND: &'static str = "cancel_stripe_subscription";
    const MAX_ATTEMPTS: i32 = RETRY_FOR_DAYS_OF_ATTEMPTS;

    async fn run(self, app: &App) -> Result<(), JobError> {
        if !is_id(&self.subscription, "sub_") {
            return Err(JobError::new("not a Stripe subscription id"));
        }
        let stripe = configured(app)?;
        let path = format!("/v1/subscriptions/{}", self.subscription);
        match stripe.send(Request::delete(&path)).await {
            Ok(_) => {}
            Err(error) if error.is_not_found() => {}
            Err(error) => return Err(JobError::new(error)),
        }
        if self.refund {
            refund_latest_invoice(stripe, &self.subscription).await?;
        }
        Ok(())
    }
}

async fn refund_latest_invoice(stripe: &Stripe, subscription: &str) -> Result<(), JobError> {
    let path = format!("/v1/invoices?subscription={subscription}&limit=1");
    let invoices = stripe
        .send(Request::get(&path))
        .await
        .map_err(JobError::new)?;
    let invoice = &invoices["data"][0];
    if invoice["amount_paid"].as_i64().unwrap_or(0) <= 0 {
        tracing::warn!(
            subscription,
            "duplicate subscription cancelled with nothing paid to refund yet"
        );
        return Ok(());
    }
    let paid_with = [
        ("payment_intent", &invoice["payment_intent"]),
        ("charge", &invoice["charge"]),
    ]
    .into_iter()
    .find_map(|(field, value)| {
        let id = expandable_id(value)?;
        Some((field, id.to_string()))
    });
    let Some((field, id)) = paid_with else {
        return Err(JobError::new("the invoice has no payment to refund"));
    };
    let form = [(field, id.as_str())];
    let key = format!("refund-{subscription}");
    let request = Request::post("/v1/refunds", &form).idempotent(Some(&key));
    match stripe.send(request).await {
        Ok(_) => Ok(()),
        Err(error) if error.has_code(ALREADY_REFUNDED) => Ok(()),
        Err(error) => Err(JobError::new(error)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateStripeCustomerEmail {
    pub customer: String,
}

impl Job for UpdateStripeCustomerEmail {
    const KIND: &'static str = "update_stripe_customer_email";

    async fn run(self, app: &App) -> Result<(), JobError> {
        if !is_id(&self.customer, "cus_") {
            return Err(JobError::new("not a Stripe customer id"));
        }
        let Some(email) = current_email(app, &self.customer).await? else {
            return Ok(());
        };
        let path = format!("/v1/customers/{}", self.customer);
        let form = [("email", email.as_str())];
        configured(app)?
            .send(Request::post(&path, &form))
            .await
            .map(|_| ())
            .map_err(JobError::new)
    }
}

async fn current_email(app: &App, customer: &str) -> Result<Option<String>, JobError> {
    let Some(user_id) = query_get_user_of_customer(customer, &app.db)
        .await
        .map_err(JobError::new)?
    else {
        return Ok(None);
    };
    let user = query_get_user_by_id(user_id, &app.db)
        .await
        .map_err(JobError::new)?;
    Ok(user.map(|user| user.email))
}

fn configured(app: &App) -> Result<&Stripe, JobError> {
    app.config
        .stripe
        .as_ref()
        .ok_or_else(|| JobError::new("billing isn't configured"))
}
