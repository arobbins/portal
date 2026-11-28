use crate::billing::jobs::is_stripe_id;
use crate::billing::queries::{query_get_customer_id, query_insert_customer};
use crate::env::Env;
use crate::stripe::{Request, Stripe, StripeError};
use crate::user::model::User;

pub async fn ensure_customer(
    env: &Env<'_>,
    stripe: &Stripe,
    user: &User,
) -> topcoat::Result<Result<String, StripeError>> {
    if let Some(customer) = query_get_customer_id(user.id, env.db()).await? {
        return Ok(Ok(customer));
    }
    let form = [
        ("email", user.email.as_str()),
        ("name", user.name.as_str()),
        ("metadata[user]", user.public_id.as_str()),
        ("metadata[app]", env.app.config.origin()),
    ];
    let key = format!("customer-{}", user.public_id);
    let request = Request::post("/v1/customers", &form).idempotent(Some(&key));
    let created = match stripe.send(request).await {
        Ok(created) => created,
        Err(error) => return Ok(Err(error)),
    };
    let Some(id) = created["id"].as_str().filter(|id| is_stripe_id(id, "cus_")) else {
        return Ok(Err(StripeError::Unreachable(
            "Stripe returned a customer without an id".to_string(),
        )));
    };
    query_insert_customer(user.id, id, env.db()).await?;
    let stored = query_get_customer_id(user.id, env.db()).await?;

    Ok(Ok(stored.unwrap_or_else(|| id.to_string())))
}
