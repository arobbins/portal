use serde_json::Value;
use sqlx::PgExecutor;

pub(super) struct ClaimedJob {
    pub id: i64,
    pub kind: String,
    pub payload: Value,
    pub attempts: i32,
    pub max_attempts: i32,
}

pub(super) async fn query_insert_job(
    kind: &str,
    payload: &Value,
    max_attempts: i32,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO jobs (kind, payload, max_attempts) VALUES ($1, $2, $3)",
        kind,
        payload,
        max_attempts,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub(super) async fn query_schedule_if_missing(
    kind: &str,
    run_at: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO jobs (kind, run_at) SELECT $1, $2 WHERE NOT EXISTS (SELECT 1 FROM jobs WHERE kind = $1 AND status = 'queued') ON CONFLICT DO NOTHING",
        kind,
        run_at,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub(super) async fn query_claim_job(
    lock_for_seconds: f64,
    db: impl PgExecutor<'_>,
) -> Result<Option<ClaimedJob>, sqlx::Error> {
    sqlx::query_as!(
        ClaimedJob,
        r#"UPDATE jobs
        SET attempts = attempts + 1, locked_until = now() + make_interval(secs => $1)
        WHERE id = (
            SELECT id FROM jobs
            WHERE status = 'queued' AND run_at <= now() AND attempts < max_attempts
              AND (locked_until IS NULL OR locked_until < now())
            ORDER BY run_at
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        RETURNING id, kind, payload, attempts, max_attempts"#,
        lock_for_seconds,
    )
    .fetch_optional(db)
    .await
}

pub(super) async fn query_fail_abandoned_jobs(
    error: &str,
    db: impl PgExecutor<'_>,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE jobs SET status = 'failed', locked_until = NULL, last_error = $1 WHERE status = 'queued' AND attempts >= max_attempts AND (locked_until IS NULL OR locked_until < now()) RETURNING kind",
        error,
    )
    .fetch_all(db)
    .await
}

pub(super) async fn query_delete_job(id: i64, db: impl PgExecutor<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM jobs WHERE id = $1", id)
        .execute(db)
        .await?;

    Ok(())
}

pub(super) async fn query_retry_job(
    id: i64,
    error: &str,
    delay_seconds: f64,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE jobs SET run_at = now() + make_interval(secs => $3), locked_until = NULL, last_error = $2 WHERE id = $1",
        id,
        error,
        delay_seconds,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub(super) async fn query_fail_job(
    id: i64,
    error: &str,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE jobs SET status = 'failed', locked_until = NULL, last_error = $2 WHERE id = $1",
        id,
        error,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_delete_old_failed_jobs(
    cutoff: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM jobs WHERE status = 'failed' AND created_at < $1",
        cutoff
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}
