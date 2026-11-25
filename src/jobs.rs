mod queue;
mod worker;

use std::future::Future;
use std::pin::Pin;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::PgExecutor;

use crate::env::App;

const FAILED_JOB_RETENTION: time::Duration = time::Duration::days(30);

pub async fn delete_old_failed_jobs(app: &App) -> Result<u64, sqlx::Error> {
    queue::query_delete_old_failed_jobs(app.clock.now() - FAILED_JOB_RETENTION, &app.db).await
}
pub use worker::Worker;

pub trait Job: Serialize + DeserializeOwned + Send + Sync + 'static {
    const KIND: &'static str;
    const REPEAT_EVERY: Option<time::Duration> = None;
    const MAX_ATTEMPTS: i32 = 5;

    fn run(self, app: &App) -> impl Future<Output = Result<(), JobError>> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobError(String);

impl JobError {
    pub fn new(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<sqlx::Error> for JobError {
    fn from(error: sqlx::Error) -> Self {
        Self::new(error)
    }
}

impl From<topcoat::Error> for JobError {
    fn from(error: topcoat::Error) -> Self {
        Self::new(error)
    }
}

pub async fn enqueue<J: Job>(job: &J, db: impl PgExecutor<'_>) -> Result<(), sqlx::Error> {
    let payload = serde_json::to_value(job).unwrap_or_else(|_| serde_json::json!({}));
    queue::query_insert_job(J::KIND, &payload, J::MAX_ATTEMPTS, db).await
}

type HandlerFuture<'a> = Pin<Box<dyn Future<Output = Result<(), JobError>> + Send + 'a>>;

pub struct Handler {
    pub kind: &'static str,
    pub repeat_every: Option<time::Duration>,
    run: for<'a> fn(&'a App, Value) -> HandlerFuture<'a>,
}

impl Handler {
    pub fn of<J: Job>() -> Self {
        Self {
            kind: J::KIND,
            repeat_every: J::REPEAT_EVERY,
            run: run_job::<J>,
        }
    }

    pub async fn run(&self, app: &App, payload: Value) -> Result<(), JobError> {
        (self.run)(app, payload).await
    }
}

fn run_job<J: Job>(app: &App, payload: Value) -> HandlerFuture<'_> {
    Box::pin(async move {
        let job: J = serde_json::from_value(payload)
            .map_err(|e| JobError::new(format!("bad `{}` payload: {e}", J::KIND)))?;
        job.run(app).await
    })
}
