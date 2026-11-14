use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgExecutor, PgPool};
use tower_sessions_sqlx_store::PostgresStore;

use crate::auth::actions::send_reset_link;
use crate::cleanup;
use crate::config::Config;
use crate::mail::Email;

const IDLE_POLL: Duration = Duration::from_millis(500);
const ERROR_PAUSE: Duration = Duration::from_secs(5);
const LOCK_FOR_SECONDS: f64 = 300.0;
const CLEANUP_EVERY: time::Duration = time::Duration::hours(1);
const MAX_BACKOFF_SECONDS: i64 = 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Job {
    SendEmail(Email),
    SendPasswordReset { email: String },
    Cleanup,
}

impl Job {
    #[cfg(test)]
    pub const KINDS: [&'static str; 3] = ["send_email", "send_password_reset", "cleanup"];

    fn kind(&self) -> &'static str {
        match self {
            Self::SendEmail(_) => "send_email",
            Self::SendPasswordReset { .. } => "send_password_reset",
            Self::Cleanup => "cleanup",
        }
    }

    fn payload(&self) -> Value {
        match self {
            Self::SendEmail(email) => serde_json::to_value(email).unwrap_or_else(|_| json!({})),
            Self::SendPasswordReset { email } => json!({ "email": email }),
            Self::Cleanup => json!({}),
        }
    }

    fn parse(kind: &str, payload: Value) -> Result<Self, String> {
        let parsed = match kind {
            "send_email" => serde_json::from_value(payload).map(Self::SendEmail),
            "send_password_reset" => serde_json::from_value::<ResetPayload>(payload)
                .map(|reset| Self::SendPasswordReset { email: reset.email }),
            "cleanup" => Ok(Self::Cleanup),
            other => return Err(format!("unknown job kind `{other}`")),
        };
        parsed.map_err(|e| format!("bad `{kind}` payload: {e}"))
    }
}

#[derive(Deserialize)]
struct ResetPayload {
    email: String,
}

pub async fn enqueue(job: &Job, db: impl PgExecutor<'_>) -> Result<(), sqlx::Error> {
    query_insert_job(
        job.kind(),
        &job.payload(),
        time::OffsetDateTime::now_utc(),
        db,
    )
    .await
}

#[derive(Clone)]
pub struct Worker {
    db: PgPool,
    config: Config,
    sessions: PostgresStore,
}

impl Worker {
    pub fn new(db: PgPool, config: Config, sessions: PostgresStore) -> Self {
        Self {
            db,
            config,
            sessions,
        }
    }

    pub fn spawn(self, count: usize) {
        for _ in 0..count {
            let worker = self.clone();
            tokio::spawn(async move { worker.run_forever().await });
        }
    }

    pub async fn run_forever(&self) {
        if let Err(e) = query_schedule_cleanup(time::OffsetDateTime::now_utc(), &self.db).await {
            tracing::error!("could not schedule cleanup: {e}");
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

    pub async fn run_next(&self) -> Result<bool, sqlx::Error> {
        let Some(claimed) = query_claim_job(LOCK_FOR_SECONDS, &self.db).await? else {
            return Ok(false);
        };
        let outcome = match Job::parse(&claimed.kind, claimed.payload) {
            Ok(job) => self.perform(&job).await,
            Err(problem) => Err(problem),
        };

        match outcome {
            Ok(()) => {
                query_delete_job(claimed.id, &self.db).await?;
                if claimed.kind == "cleanup" {
                    let next = time::OffsetDateTime::now_utc() + CLEANUP_EVERY;
                    query_schedule_cleanup(next, &self.db).await?;
                }
            }
            Err(problem) if claimed.attempts >= claimed.max_attempts => {
                tracing::error!(job = claimed.id, kind = %claimed.kind, "job failed for good: {problem}");
                query_fail_job(claimed.id, &problem, &self.db).await?;
            }
            Err(problem) => {
                tracing::warn!(job = claimed.id, kind = %claimed.kind, attempt = claimed.attempts, "job failed, will retry: {problem}");
                query_retry_job(
                    claimed.id,
                    &problem,
                    backoff_seconds(claimed.attempts),
                    &self.db,
                )
                .await?;
            }
        }

        Ok(true)
    }

    async fn perform(&self, job: &Job) -> Result<(), String> {
        match job {
            Job::SendEmail(email) => self
                .config
                .mailer
                .send(email)
                .await
                .map_err(|e| e.to_string()),
            Job::SendPasswordReset { email } => send_reset_link(&self.db, &self.config, email)
                .await
                .map_err(|e| e.to_string()),
            Job::Cleanup => cleanup::run(&self.db, &self.sessions)
                .await
                .map_err(|e| e.to_string()),
        }
    }
}

fn backoff_seconds(attempts: i32) -> f64 {
    let exponent = attempts.clamp(0, 20) as u32;
    (30_i64 * 2_i64.pow(exponent)).min(MAX_BACKOFF_SECONDS) as f64
}

struct ClaimedJob {
    id: i64,
    kind: String,
    payload: Value,
    attempts: i32,
    max_attempts: i32,
}

async fn query_insert_job(
    kind: &str,
    payload: &Value,
    run_at: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO jobs (kind, payload, run_at) VALUES ($1, $2, $3)",
        kind,
        payload,
        run_at,
    )
    .execute(db)
    .await?;

    Ok(())
}

async fn query_schedule_cleanup(
    run_at: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO jobs (kind, run_at) VALUES ('cleanup', $1) ON CONFLICT (kind) WHERE kind = 'cleanup' AND status = 'queued' DO NOTHING",
        run_at,
    )
    .execute(db)
    .await?;

    Ok(())
}

async fn query_claim_job(
    lock_for_seconds: f64,
    db: impl PgExecutor<'_>,
) -> Result<Option<ClaimedJob>, sqlx::Error> {
    sqlx::query_as!(
        ClaimedJob,
        r#"UPDATE jobs
        SET attempts = attempts + 1, locked_until = now() + make_interval(secs => $1)
        WHERE id = (
            SELECT id FROM jobs
            WHERE status = 'queued' AND run_at <= now()
              AND (locked_until IS NULL OR locked_until < now())
            ORDER BY run_at
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        RETURNING id, kind, payload, attempts, max_attempts"#,
        lock_for_seconds,
    )
    .fetch_optional(db)
    .await
}

async fn query_delete_job(id: i64, db: impl PgExecutor<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM jobs WHERE id = $1", id)
        .execute(db)
        .await?;

    Ok(())
}

async fn query_retry_job(
    id: i64,
    error: &str,
    delay_seconds: f64,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE jobs SET run_at = now() + make_interval(secs => $3), locked_until = NULL, last_error = $2 WHERE id = $1",
        id,
        error,
        delay_seconds,
    )
    .execute(db)
    .await?;

    Ok(())
}

async fn query_fail_job(id: i64, error: &str, db: impl PgExecutor<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE jobs SET status = 'failed', locked_until = NULL, last_error = $2 WHERE id = $1",
        id,
        error,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_delete_old_failed_jobs(
    older_than: time::Duration,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let cutoff = time::OffsetDateTime::now_utc() - older_than;
    let result = sqlx::query!(
        "DELETE FROM jobs WHERE status = 'failed' AND created_at < $1",
        cutoff
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker(db: &PgPool) -> Worker {
        let config = Config::from_lookup(|key| match key {
            "APP_BASE_URL" => Some("http://localhost".to_string()),
            "MAIL_TRANSPORT" => Some("log".to_string()),
            _ => None,
        })
        .unwrap();
        Worker::new(db.clone(), config, PostgresStore::new(db.clone()))
    }

    fn email() -> Email {
        Email {
            to: "someone@example.test".to_string(),
            subject: "Hello".to_string(),
            text: "Hi".to_string(),
            html: "<p>Hi</p>".to_string(),
        }
    }

    #[test]
    fn every_job_round_trips_through_its_row() {
        for job in [
            Job::SendEmail(email()),
            Job::SendPasswordReset {
                email: "a@example.test".to_string(),
            },
            Job::Cleanup,
        ] {
            assert!(Job::KINDS.contains(&job.kind()));
            assert_eq!(Job::parse(job.kind(), job.payload()).unwrap(), job);
        }
    }

    #[test]
    fn retries_back_off_up_to_an_hour() {
        assert_eq!(backoff_seconds(1), 60.0);
        assert_eq!(backoff_seconds(2), 120.0);
        assert_eq!(backoff_seconds(20), MAX_BACKOFF_SECONDS as f64);
    }

    #[sqlx::test]
    async fn a_finished_job_is_removed_and_cleanup_reschedules_itself(db: PgPool) {
        enqueue(&Job::SendEmail(email()), &db).await.unwrap();
        query_schedule_cleanup(time::OffsetDateTime::now_utc(), &db)
            .await
            .unwrap();
        let worker = worker(&db);

        assert!(worker.run_next().await.unwrap());
        assert!(worker.run_next().await.unwrap());
        assert!(!worker.run_next().await.unwrap());

        let left = sqlx::query!("SELECT kind, run_at > now() AS \"later!\" FROM jobs")
            .fetch_all(&db)
            .await
            .unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].kind, "cleanup");
        assert!(left[0].later);
    }

    #[sqlx::test]
    async fn a_broken_job_is_retried_then_marked_failed(db: PgPool) {
        sqlx::query!(
            "INSERT INTO jobs (kind, payload, max_attempts) VALUES ('send_email', '{}', 2)"
        )
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

        let job = sqlx::query!("SELECT status, attempts, last_error FROM jobs")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!((job.status.as_str(), job.attempts), ("failed", 2));
        assert!(job.last_error.unwrap().contains("send_email"));
    }

    #[sqlx::test]
    async fn only_one_cleanup_is_ever_queued(db: PgPool) {
        let now = time::OffsetDateTime::now_utc();
        query_schedule_cleanup(now, &db).await.unwrap();
        query_schedule_cleanup(now, &db).await.unwrap();

        let count = sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM jobs"#)
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
