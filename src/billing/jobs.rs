use serde::{Deserialize, Serialize};

use crate::env::App;
use crate::jobs::{Job, JobError};
use crate::stripe::{Request, Stripe};

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
        if !is_stripe_id(&self.subscription, "sub_") {
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
        let id = value.as_str().or_else(|| value["id"].as_str())?;
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
    pub email: String,
}

impl Job for UpdateStripeCustomerEmail {
    const KIND: &'static str = "update_stripe_customer_email";

    async fn run(self, app: &App) -> Result<(), JobError> {
        if !is_stripe_id(&self.customer, "cus_") {
            return Err(JobError::new("not a Stripe customer id"));
        }
        let path = format!("/v1/customers/{}", self.customer);
        let form = [("email", self.email.as_str())];
        configured(app)?
            .send(Request::post(&path, &form))
            .await
            .map(|_| ())
            .map_err(JobError::new)
    }
}

fn configured(app: &App) -> Result<&Stripe, JobError> {
    app.config
        .stripe
        .as_ref()
        .ok_or_else(|| JobError::new("billing isn't configured"))
}

pub fn is_stripe_id(value: &str, prefix: &str) -> bool {
    value.len() <= 255
        && value.strip_prefix(prefix).is_some_and(|rest| {
            !rest.is_empty()
                && rest
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
}
