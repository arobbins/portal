use super::queries::query_list_user_api_feedback;
use crate::data_export::{Dataset, utc};
use crate::env::Env;
use crate::user::model::User;

pub async fn api_feedback_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let feedback = query_list_user_api_feedback(user.id, env.db()).await?;
    Ok(Dataset {
        file: "api_feedback.csv",
        about: "Feedback you sent through the API.",
        tables: &["api_feedback"],
        columns: &[
            "id",
            "category",
            "endpoint",
            "problem_code",
            "request_id",
            "message",
            "sent_at",
        ],
        rows: feedback
            .into_iter()
            .map(|row| {
                vec![
                    row.public_id,
                    row.category.as_str().to_string(),
                    row.endpoint.unwrap_or_default(),
                    row.problem_code.unwrap_or_default(),
                    row.request_id.unwrap_or_default(),
                    row.message,
                    utc(row.created_at),
                ]
            })
            .collect(),
    })
}
