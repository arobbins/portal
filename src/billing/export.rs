use crate::billing::actions::billing_of;
use crate::billing::queries::{query_get_current_subscription, query_get_customer_id};
use crate::data_export::Dataset;
use crate::env::Env;
use crate::user::model::User;

pub async fn billing_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let current = query_get_current_subscription(user.id, env.db()).await?;
    let customer = query_get_customer_id(user.id, env.db()).await?;
    let billing = billing_of(current.as_ref(), &user.zone());
    Ok(Dataset {
        file: "billing.csv",
        about: "Your plan, as Stripe last told us: its status (free, active, cancelling or past_due), the day it renews or ends (YYYY-MM-DD in your time zone), and your Stripe customer id. Card, payment and invoice details stay in Stripe: open Manage billing to see or download them.",
        tables: &["billing_customers", "subscriptions"],
        columns: &[
            "plan",
            "status",
            "renews_on",
            "ends_on",
            "stripe_customer_id",
        ],
        rows: vec![vec![
            billing.plan,
            billing.status.as_str().to_string(),
            billing.renews_on.unwrap_or_default(),
            billing.ends_on.unwrap_or_default(),
            customer.unwrap_or_default(),
        ]],
    })
}

#[cfg(test)]
mod tests;
