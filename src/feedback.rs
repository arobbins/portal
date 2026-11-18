pub mod actions;
pub mod api;
mod queries;
mod redact;

use crate::env::App;

pub const FEEDBACK_RETENTION: time::Duration = time::Duration::days(90);

pub async fn delete_old_feedback(app: &App) -> Result<u64, sqlx::Error> {
    queries::query_delete_old_api_feedback(app.clock.now() - FEEDBACK_RETENTION, &app.db).await
}
