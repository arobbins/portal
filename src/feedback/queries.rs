use sqlx::PgExecutor;
use time::OffsetDateTime;

use super::actions::FeedbackCategory;

pub struct NewFeedback<'a> {
    pub public_id: &'a str,
    pub user_id: i32,
    pub api_token_id: Option<i32>,
    pub category: FeedbackCategory,
    pub endpoint: Option<&'a str>,
    pub problem_code: Option<&'a str>,
    pub request_id: Option<&'a str>,
    pub message: &'a str,
}

pub async fn query_insert_api_feedback(
    feedback: &NewFeedback<'_>,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO api_feedback (public_id, user_id, token_public_id, category, endpoint, problem_code, request_id, message)
         VALUES ($1, $2, (SELECT public_id FROM api_tokens WHERE id = $3 AND user_id = $2), $4, $5, $6, $7, $8)",
        feedback.public_id,
        feedback.user_id,
        feedback.api_token_id,
        feedback.category.as_str(),
        feedback.endpoint,
        feedback.problem_code,
        feedback.request_id,
        feedback.message,
    )
    .execute(db)
    .await?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackRow {
    pub public_id: String,
    pub email: String,
    pub token_public_id: Option<String>,
    pub category: FeedbackCategory,
    pub endpoint: Option<String>,
    pub problem_code: Option<String>,
    pub request_id: Option<String>,
    pub message: String,
    pub created_at: OffsetDateTime,
}

pub async fn query_list_recent_api_feedback(
    limit: i64,
    db: impl PgExecutor<'_>,
) -> Result<Vec<FeedbackRow>, sqlx::Error> {
    sqlx::query_as!(
        FeedbackRow,
        r#"SELECT f.public_id, u.email, f.token_public_id, f.category AS "category: FeedbackCategory",
                  f.endpoint, f.problem_code, f.request_id, f.message, f.created_at
           FROM api_feedback f JOIN users u ON u.id = f.user_id
           ORDER BY f.created_at DESC, f.id DESC
           LIMIT $1"#,
        limit,
    )
    .fetch_all(db)
    .await
}

pub async fn query_list_user_api_feedback(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Vec<FeedbackRow>, sqlx::Error> {
    sqlx::query_as!(
        FeedbackRow,
        r#"SELECT f.public_id, u.email, f.token_public_id, f.category AS "category: FeedbackCategory",
                  f.endpoint, f.problem_code, f.request_id, f.message, f.created_at
           FROM api_feedback f JOIN users u ON u.id = f.user_id
           WHERE f.user_id = $1
           ORDER BY f.created_at DESC, f.id DESC"#,
        user_id,
    )
    .fetch_all(db)
    .await
}

pub async fn query_delete_old_api_feedback(
    cutoff: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM api_feedback WHERE created_at < $1", cutoff)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}
