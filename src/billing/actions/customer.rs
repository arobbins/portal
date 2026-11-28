use super::stripe_call::StripeCall;
use crate::billing::queries::{query_get_customer_id, query_insert_customer};
use crate::env::Env;
use crate::stripe::is_id;
use crate::stripe::{Request, Stripe, StripeError};
use crate::user::model::User;

pub async fn ensure_customer(env: &Env<'_>, stripe: &Stripe, user: &User) -> StripeCall<String> {
    if let Some(customer) = query_get_customer_id(user.id, env.db()).await? {
        return Ok(customer);
    }
    let form = [
        ("email", user.email.as_str()),
        ("name", user.name.as_str()),
        ("metadata[user]", user.public_id.as_str()),
        ("metadata[app]", env.app.config.origin()),
    ];
    let key = format!("customer-{}", user.public_id);
    let request = Request::post("/v1/customers", &form).idempotent(Some(&key));
    let created = stripe.send(request).await?;
    let Some(id) = created["id"].as_str().filter(|id| is_id(id, "cus_")) else {
        return Err(StripeError::Unreachable(
            "Stripe returned a customer without an id".to_string(),
        )
        .into());
    };
    query_insert_customer(user.id, id, env.db()).await?;
    let stored = query_get_customer_id(user.id, env.db()).await?;

    Ok(stored.unwrap_or_else(|| id.to_string()))
}
