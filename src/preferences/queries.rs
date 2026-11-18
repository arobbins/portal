use sqlx::PgExecutor;

use super::notifications::NotificationTopic;

pub async fn query_get_opt_out(
    user_id: i32,
    topic: NotificationTopic,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let found = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM notification_opt_outs WHERE user_id = $1 AND topic = $2) AS "found!""#,
        user_id,
        topic as NotificationTopic,
    )
    .fetch_one(db)
    .await?;

    Ok(found)
}

pub async fn query_insert_opt_out(
    user_id: i32,
    topic: NotificationTopic,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO notification_opt_outs (user_id, topic) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        user_id,
        topic as NotificationTopic,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_delete_opt_out(
    user_id: i32,
    topic: NotificationTopic,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM notification_opt_outs WHERE user_id = $1 AND topic = $2",
        user_id,
        topic as NotificationTopic,
    )
    .execute(db)
    .await?;

    Ok(())
}
