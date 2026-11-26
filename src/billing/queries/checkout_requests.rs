use sqlx::PgExecutor;
use time::OffsetDateTime;

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
