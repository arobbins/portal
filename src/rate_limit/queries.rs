use sqlx::PgExecutor;

pub(super) async fn query_lock_rate_limit_key(
    bucket: &str,
    key: &str,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT true AS "locked!" FROM pg_advisory_xact_lock(hashtextextended($1 || ' ' || $2, 0))"#,
        bucket,
        key,
    )
    .fetch_one(db)
    .await?;

    Ok(())
}

pub(super) async fn query_take_rate_limit_slot(
    bucket: &str,
    key: &str,
    window_seconds: f64,
    max: i64,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let taken = sqlx::query_scalar!(
        r#"WITH recent AS (
            SELECT count(*) AS hits FROM rate_limit_hits
            WHERE bucket = $1 AND key = $2 AND at > now() - make_interval(secs => $3)
        )
        INSERT INTO rate_limit_hits (bucket, key)
        SELECT $1, $2 FROM recent WHERE recent.hits < $4
        RETURNING 1 AS "taken!""#,
        bucket,
        key,
        window_seconds,
        max,
    )
    .fetch_optional(db)
    .await?;

    Ok(taken.is_some())
}

pub(super) async fn query_count_rate_limit_hits(
    bucket: &str,
    key: &str,
    window_seconds: f64,
    db: impl PgExecutor<'_>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "hits!" FROM rate_limit_hits
        WHERE bucket = $1 AND key = $2 AND at > now() - make_interval(secs => $3)"#,
        bucket,
        key,
        window_seconds,
    )
    .fetch_one(db)
    .await
}

pub(super) async fn query_insert_rate_limit_hit(
    bucket: &str,
    key: &str,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO rate_limit_hits (bucket, key) VALUES ($1, $2)",
        bucket,
        key,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub(super) async fn query_delete_rate_limit_hits(
    bucket: &str,
    key: &str,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM rate_limit_hits WHERE bucket = $1 AND key = $2",
        bucket,
        key,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub(super) async fn query_delete_newest_rate_limit_hit(
    bucket: &str,
    key: &str,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM rate_limit_hits WHERE ctid = (
            SELECT ctid FROM rate_limit_hits WHERE bucket = $1 AND key = $2
            ORDER BY at DESC LIMIT 1
        )",
        bucket,
        key,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub(super) async fn query_delete_old_rate_limit_hits(
    cutoff: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM rate_limit_hits WHERE at < $1", cutoff)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}
