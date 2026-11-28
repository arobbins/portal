use serde::{Deserialize, Serialize};
use tower_sessions::session_store::ExpiredDeletion;
use tower_sessions_sqlx_store::PostgresStore;

use crate::api_tokens::lifecycle::delete_long_expired;
use crate::auth::links::delete_stale_sign_ups;
use crate::billing::delete_expired as delete_expired_billing_records;
use crate::devices::known::forget_unseen_devices;
use crate::devices::lifecycle::delete_stale_devices;
use crate::env::App;
use crate::feedback::delete_old_feedback;
use crate::jobs::{Job, JobError, delete_old_failed_jobs};
use crate::oauth::lifecycle::delete_expired as delete_expired_sign_ins;
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

#[derive(Default)]
struct Steps {
    failed: Vec<&'static str>,
}

impl Steps {
    fn count<T: Default, E: std::fmt::Display>(
        &mut self,
        step: &'static str,
        result: Result<T, E>,
    ) -> T {
        result.unwrap_or_else(|error| {
            tracing::error!(step, "cleanup step failed: {error}");
            self.failed.push(step);
            T::default()
        })
    }

    fn finish(self) -> Result<(), JobError> {
        if self.failed.is_empty() {
            return Ok(());
        }
        Err(JobError::new(format!(
            "cleanup steps failed: {}",
            self.failed.join(", ")
        )))
    }
}

async fn run(app: &App) -> Result<(), JobError> {
    let mut steps = Steps::default();
    let (links, sign_ups) = steps.count(
        "expired links and abandoned sign-ups",
        delete_stale_sign_ups(app).await,
    );
    let api_tokens = steps.count("expired API tokens", delete_long_expired(app).await);
    let hits = steps.count(
        "rate-limit hits",
        app.limits.forget_old_hits(app.clock.now()).await,
    );
    let events = steps.count("old account events", delete_old_events(app).await);
    let jobs = steps.count("failed jobs", delete_old_failed_jobs(app).await);
    let devices = steps.count("signed-out devices", delete_stale_devices(app).await);
    let known = steps.count("long-unseen browsers", forget_unseen_devices(app).await);
    let feedback = steps.count("old API feedback", delete_old_feedback(app).await);
    let (stripe_events, checkouts) = steps.count(
        "old billing records",
        delete_expired_billing_records(app).await,
    );
    let sign_ins = steps.count("expired app sign-ins", delete_expired_sign_ins(app).await);
    let store = PostgresStore::new(app.db.clone());
    steps.count("expired sessions", store.delete_expired().await);

    tracing::info!(
        "cleanup removed {links} expired links, {sign_ups} abandoned sign-ups, {api_tokens} expired API tokens, {hits} rate-limit hits, {events} old account events, {jobs} failed jobs, {devices} signed-out devices, {known} long-unseen browsers, {feedback} old API feedback messages, {stripe_events} old Stripe event ids, {checkouts} old checkout requests and {sign_ins} expired app sign-in records"
    );
    steps.finish()
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

    #[sqlx::test]
    async fn a_failing_step_does_not_skip_the_later_ones(db: PgPool) {
        let store = session_store(&db).await;
        save_session(&store, time::Duration::hours(-1)).await;
        sqlx::query("ALTER TABLE user_tokens RENAME TO user_tokens_gone")
            .execute(&db)
            .await
            .unwrap();

        let error = run(&App::for_tests(db.clone())).await.unwrap_err();

        assert_eq!(session_count(&db).await, 0);
        assert!(
            error
                .to_string()
                .contains("expired links and abandoned sign-ups")
        );
    }
}
