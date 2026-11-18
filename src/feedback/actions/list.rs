use std::convert::Infallible;

use time::OffsetDateTime;

use super::FeedbackCategory;
use crate::action::{ActionResult, Caller, authorize};
use crate::env::Env;
use crate::feedback::queries::{FeedbackRow, query_list_recent_api_feedback};
use crate::permission::Permission;

pub const FEEDBACK_SHOWN: i64 = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackView {
    pub id: String,
    pub from: String,
    pub token: Option<String>,
    pub category: FeedbackCategory,
    pub endpoint: Option<String>,
    pub problem_code: Option<String>,
    pub request_id: Option<String>,
    pub message: String,
    pub sent_at: OffsetDateTime,
}

impl From<FeedbackRow> for FeedbackView {
    fn from(row: FeedbackRow) -> Self {
        Self {
            id: row.public_id,
            from: row.email,
            token: row.token_public_id,
            category: row.category,
            endpoint: row.endpoint,
            problem_code: row.problem_code,
            request_id: row.request_id,
            message: row.message,
            sent_at: row.created_at,
        }
    }
}

pub async fn list_api_feedback(
    env: &Env<'_>,
    caller: &Caller,
) -> ActionResult<Vec<FeedbackView>, Infallible> {
    authorize!(caller.require(Permission::AdministerUsers));
    let rows = query_list_recent_api_feedback(FEEDBACK_SHOWN, env.db()).await?;

    Ok(Ok(rows.into_iter().map(FeedbackView::from).collect()))
}
