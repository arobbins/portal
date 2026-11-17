use std::sync::Arc;
use std::time::Duration;

use super::queue::{
    ClaimedJob, query_claim_job, query_delete_job, query_fail_abandoned_jobs, query_fail_job,
    query_retry_job, query_schedule_if_missing,
};
use super::{Handler, JobError};
use crate::env::App;

const IDLE_POLL: Duration = Duration::from_millis(500);
const ERROR_PAUSE: Duration = Duration::from_secs(5);
const LOCK_FOR_SECONDS: f64 = 300.0;
const MAX_BACKOFF_SECONDS: i64 = 60 * 60;
const ABANDONED: &str = "A worker stopped during the job's last attempt.";

#[derive(Clone)]
pub struct Worker {
    app: App,
    handlers: Arc<Vec<Handler>>,
}

impl Worker {
    pub fn new(app: App, handlers: Vec<Handler>) -> Self {
        Self {
            app,
            handlers: Arc::new(handlers),
        }
    }

    pub fn spawn(&self, count: usize) {
        for _ in 0..count {
            let worker = self.clone();
            tokio::spawn(async move { worker.run_forever().await });
        }
    }

    async fn run_forever(&self) {
        if let Err(e) = self.schedule_repeating_jobs().await {
            tracing::error!("could not schedule repeating jobs: {e}");
        }
        loop {
            match self.run_next().await {
                Ok(true) => {}
                Ok(false) => tokio::time::sleep(IDLE_POLL).await,
                Err(e) => {
                    tracing::error!("job queue unavailable: {e}");
                    tokio::time::sleep(ERROR_PAUSE).await;
                }
            }
        }
    }

    async fn schedule_repeating_jobs(&self) -> Result<(), sqlx::Error> {
        let now = self.app.clock.now();
        for handler in self.handlers.iter().filter(|h| h.repeat_every.is_some()) {
            query_schedule_if_missing(handler.kind, now, &self.app.db).await?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub async fn claim_and_stop(&self) -> Result<bool, sqlx::Error> {
        Ok(query_claim_job(LOCK_FOR_SECONDS, &self.app.db)
            .await?
            .is_some())
    }

    pub async fn run_next(&self) -> Result<bool, sqlx::Error> {
        let abandoned = query_fail_abandoned_jobs(ABANDONED, &self.app.db).await?;
        if abandoned > 0 {
            tracing::error!(count = abandoned, "{ABANDONED}");
        }
        let Some(claimed) = query_claim_job(LOCK_FOR_SECONDS, &self.app.db).await? else {
            return Ok(abandoned > 0);
        };
        let outcome = self.perform(&claimed).await;
        self.settle(&claimed, outcome).await?;
        Ok(true)
    }

    async fn perform(&self, claimed: &ClaimedJob) -> Result<(), JobError> {
        let handler = self.handler(&claimed.kind)?;
        handler.run(&self.app, claimed.payload.clone()).await
    }

    fn handler(&self, kind: &str) -> Result<&Handler, JobError> {
        self.handlers
            .iter()
            .find(|handler| handler.kind == kind)
            .ok_or_else(|| JobError::new(format!("no handler for job kind `{kind}`")))
    }

    async fn settle(
        &self,
        claimed: &ClaimedJob,
        outcome: Result<(), JobError>,
    ) -> Result<(), sqlx::Error> {
        let db = &self.app.db;
        match outcome {
            Ok(()) => {
                query_delete_job(claimed.id, db).await?;
                self.repeat(&claimed.kind).await
            }
            Err(error) if claimed.attempts >= claimed.max_attempts => {
                tracing::error!(job = claimed.id, kind = %claimed.kind, "job failed for good: {error}");
                query_fail_job(claimed.id, &error.to_string(), db).await
            }
            Err(error) => {
                tracing::warn!(job = claimed.id, kind = %claimed.kind, attempt = claimed.attempts, "job failed, will retry: {error}");
                let delay = backoff_seconds(claimed.attempts);
                query_retry_job(claimed.id, &error.to_string(), delay, db).await
            }
        }
    }

    async fn repeat(&self, kind: &str) -> Result<(), sqlx::Error> {
        let Ok(handler) = self.handler(kind) else {
            return Ok(());
        };
        let Some(every) = handler.repeat_every else {
            return Ok(());
        };
        query_schedule_if_missing(kind, self.app.clock.now() + every, &self.app.db).await
    }
}

fn backoff_seconds(attempts: i32) -> f64 {
    let exponent = attempts.clamp(0, 20) as u32;
    (30_i64 * 2_i64.pow(exponent)).min(MAX_BACKOFF_SECONDS) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::{Job, enqueue};
    use serde::{Deserialize, Serialize};
    use sqlx::PgPool;

    #[derive(Serialize, Deserialize)]
    struct Succeeds {}

    impl Job for Succeeds {
        const KIND: &'static str = "send_email";
        const REPEAT_EVERY: Option<time::Duration> = Some(time::Duration::hours(1));

        async fn run(self, _app: &App) -> Result<(), JobError> {
            Ok(())
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Fails {}

    impl Job for Fails {
        const KIND: &'static str = "cleanup";

        async fn run(self, _app: &App) -> Result<(), JobError> {
            Err(JobError::new("boom"))
        }
    }

    fn worker(db: &PgPool) -> Worker {
        Worker::new(
            App::for_tests(db.clone()),
            vec![Handler::of::<Succeeds>(), Handler::of::<Fails>()],
        )
    }

    async fn jobs(db: &PgPool) -> Vec<(String, String, i32, bool)> {
        sqlx::query!(
            r#"SELECT kind, status, attempts, run_at > now() AS "later!" FROM jobs ORDER BY id"#
        )
        .fetch_all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.kind, row.status, row.attempts, row.later))
        .collect()
    }

    #[test]
    fn retries_back_off_up_to_an_hour() {
        assert_eq!(backoff_seconds(1), 60.0);
        assert_eq!(backoff_seconds(2), 120.0);
        assert_eq!(backoff_seconds(20), MAX_BACKOFF_SECONDS as f64);
    }

    #[sqlx::test]
    async fn a_finished_repeating_job_is_replaced_by_its_next_run(db: PgPool) {
        enqueue(&Succeeds {}, &db).await.unwrap();

        assert!(worker(&db).run_next().await.unwrap());

        assert_eq!(
            jobs(&db).await,
            vec![("send_email".to_string(), "queued".to_string(), 0, true)]
        );
    }

    #[sqlx::test]
    async fn a_failing_job_is_retried_then_marked_failed(db: PgPool) {
        sqlx::query!("INSERT INTO jobs (kind, payload, max_attempts) VALUES ('cleanup', '{}', 2)")
            .execute(&db)
            .await
            .unwrap();
        let worker = worker(&db);

        assert!(worker.run_next().await.unwrap());
        sqlx::query!("UPDATE jobs SET run_at = now()")
            .execute(&db)
            .await
            .unwrap();
        assert!(worker.run_next().await.unwrap());

        assert_eq!(
            jobs(&db).await,
            vec![("cleanup".to_string(), "failed".to_string(), 2, false)]
        );
        assert!(!worker.run_next().await.unwrap());
    }

    #[sqlx::test]
    async fn repeating_jobs_are_scheduled_once(db: PgPool) {
        let worker = worker(&db);
        worker.schedule_repeating_jobs().await.unwrap();
        worker.schedule_repeating_jobs().await.unwrap();

        assert_eq!(jobs(&db).await.len(), 1);
    }
}
