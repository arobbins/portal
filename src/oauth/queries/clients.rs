use sqlx::PgExecutor;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthClient {
    pub id: i32,
    pub client_id: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub fetched_at: Option<OffsetDateTime>,
}

pub struct NewClient<'a> {
    pub client_id: &'a str,
    pub name: &'a str,
    pub redirect_uris: &'a [String],
    pub fetched_at: Option<OffsetDateTime>,
}

pub async fn query_insert_client(
    client: &NewClient<'_>,
    db: impl PgExecutor<'_>,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO oauth_clients (client_id, name, redirect_uris, fetched_at) VALUES ($1, $2, $3, $4) ON CONFLICT (client_id) DO UPDATE SET name = EXCLUDED.name, redirect_uris = EXCLUDED.redirect_uris, fetched_at = EXCLUDED.fetched_at RETURNING id",
        client.client_id,
        client.name,
        client.redirect_uris,
        client.fetched_at,
    )
    .fetch_one(db)
    .await
}

pub async fn query_get_client(
    client_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<OAuthClient>, sqlx::Error> {
    sqlx::query_as!(
        OAuthClient,
        "SELECT id, client_id, name, redirect_uris, fetched_at FROM oauth_clients WHERE client_id = $1",
        client_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_record_client_approval(
    id: i32,
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE oauth_clients SET approved_at = $2 WHERE id = $1 AND approved_at IS NULL",
        id,
        now,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn query_delete_unapproved_clients(
    cutoff: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM oauth_clients WHERE approved_at IS NULL AND created_at < $1",
        cutoff,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}
