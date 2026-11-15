use serde::{Deserialize, Serialize};
use tower_sessions::session_store::ExpiredDeletion;
use tower_sessions_sqlx_store::PostgresStore;

use crate::api_tokens::lifecycle::delete_long_expired;
use crate::auth::links::delete_stale_sign_ups;
use crate::env::App;
use crate::jobs::{Job, JobError, delete_old_failed_jobs};
use crate::user::events::delete_old_events;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupJob {}

impl Job for CleanupJob {
    const KIND: &'static str = "cleanup";
    const REPEAT_EVERY: Option<time::Duration> = Some(time::Duration::hours(1));

    async fn run(self, app: &App) -> Result<(), JobError> {
        run(app).await
    }
}

async fn run(app: &App) -> Result<(), JobError> {
    let (links, sign_ups) = delete_stale_sign_ups(app).await?;
    let api_tokens = delete_long_expired(app).await?;
    let hits = app.limits.forget_old_hits(app.clock.now()).await?;
    let events = delete_old_events(app).await?;
    let jobs = delete_old_failed_jobs(app).await?;
    PostgresStore::new(app.db.clone())
        .delete_expired()
        .await
        .map_err(JobError::new)?;

    tracing::info!(
        "cleanup removed {links} expired links, {sign_ups} abandoned sign-ups, {api_tokens} expired API tokens, {hits} rate-limit hits, {events} old account events and {jobs} failed jobs"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use tower_sessions::session::{Id, Record};
    use tower_sessions::session_store::SessionStore;

    async fn session_store(db: &PgPool) -> PostgresStore {
        let store = PostgresStore::new(db.clone());
        store.migrate().await.unwrap();
        store
    }

    async fn save_session(store: &PostgresStore, expires_in: time::Duration) {
        let record = Record {
            id: Id::default(),
            data: Default::default(),
            expiry_date: time::OffsetDateTime::now_utc() + expires_in,
        };
        store.save(&record).await.unwrap();
    }

    async fn session_count(db: &PgPool) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM tower_sessions.session")
            .fetch_one(db)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn removes_expired_sessions(db: PgPool) {
        let store = session_store(&db).await;
        save_session(&store, time::Duration::hours(-1)).await;
        save_session(&store, time::Duration::hours(1)).await;

        run(&App::for_tests(db.clone())).await.unwrap();

        assert_eq!(session_count(&db).await, 1);
    }
}
