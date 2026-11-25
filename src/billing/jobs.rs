use serde::{Deserialize, Serialize};

use crate::env::App;
use crate::jobs::{Job, JobError};
use crate::stripe::{Request, Stripe};

const RETRY_FOR_DAYS_OF_ATTEMPTS: i32 = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelStripeSubscription {
    pub subscription: String,
}

impl Job for CancelStripeSubscription {
    const KIND: &'static str = "cancel_stripe_subscription";
    const MAX_ATTEMPTS: i32 = RETRY_FOR_DAYS_OF_ATTEMPTS;

    async fn run(self, app: &App) -> Result<(), JobError> {
        if !is_stripe_id(&self.subscription, "sub_") {
            return Err(JobError::new("not a Stripe subscription id"));
        }
        let path = format!("/v1/subscriptions/{}", self.subscription);
        match configured(app)?.send(Request::delete(&path)).await {
            Ok(_) => Ok(()),
            Err(error) if error.is_not_found() => Ok(()),
            Err(error) => Err(JobError::new(error)),
        }
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
