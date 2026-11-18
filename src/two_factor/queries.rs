use sqlx::PgExecutor;
use time::OffsetDateTime;

pub struct TwoFactorRow {
    pub secret: String,
    pub enabled_at: OffsetDateTime,
}

pub async fn query_get_two_factor(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<TwoFactorRow>, sqlx::Error> {
    sqlx::query_as!(
        TwoFactorRow,
        "SELECT secret, enabled_at FROM user_two_factor WHERE user_id = $1",
        user_id
    )
    .fetch_optional(db)
    .await
}

pub async fn query_insert_two_factor(
    user_id: i32,
    secret: &str,
    step: i64,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let inserted = sqlx::query!(
        "INSERT INTO user_two_factor (user_id, secret, last_used_step) VALUES ($1, $2, $3) ON CONFLICT (user_id) DO NOTHING",
        user_id,
        secret,
        step,
    )
    .execute(db)
    .await?;

    Ok(inserted.rows_affected() == 1)
}

pub async fn query_use_step(
    user_id: i32,
    step: i64,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let used = sqlx::query!(
        "UPDATE user_two_factor SET last_used_step = $2 WHERE user_id = $1 AND last_used_step < $2",
        user_id,
        step,
    )
    .execute(db)
    .await?;

    Ok(used.rows_affected() == 1)
}

pub async fn query_delete_two_factor(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let deleted = sqlx::query!("DELETE FROM user_two_factor WHERE user_id = $1", user_id)
        .execute(db)
        .await?;

    Ok(deleted.rows_affected() == 1)
}

pub async fn query_delete_recovery_codes(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM recovery_codes WHERE user_id = $1", user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn query_insert_recovery_codes(
    user_id: i32,
    code_hashes: &[String],
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO recovery_codes (user_id, code_hash) SELECT $1, unnest($2::TEXT[])",
        user_id,
        code_hashes,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_use_recovery_code(
    user_id: i32,
    code_hash: &str,
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let used = sqlx::query!(
        "UPDATE recovery_codes SET used_at = $3 WHERE user_id = $1 AND code_hash = $2 AND used_at IS NULL",
        user_id,
        code_hash,
        now,
    )
    .execute(db)
    .await?;

    Ok(used.rows_affected() == 1)
}

pub async fn query_count_recovery_codes_left(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM recovery_codes WHERE user_id = $1 AND used_at IS NULL"#,
        user_id
    )
    .fetch_one(db)
    .await
}
