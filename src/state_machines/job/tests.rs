use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::Duration;

use super::*;
use crate::env::{App, Clock};
use crate::jobs::{Handler, Job, JobError, Worker, delete_old_failed_jobs, enqueue};
use crate::state_machines::{CellResult, failures, write_results};

static SUCCEED: AtomicBool = AtomicBool::new(true);
static RUNS: AtomicUsize = AtomicUsize::new(0);

#[derive(Serialize, Deserialize)]
struct Scripted {}

impl Job for Scripted {
    const KIND: &'static str = "send_email";

    async fn run(self, _app: &App) -> Result<(), JobError> {
        RUNS.fetch_add(1, Ordering::SeqCst);
        if SUCCEED.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(JobError::new("the work failed"))
        }
    }
}

#[derive(Debug)]
struct Row {
    failed: bool,
    held: bool,
    later: bool,
    attempts: i32,
    max_attempts: i32,
    has_error: bool,
}

struct Bench {
    app: App,
    worker: Worker,
    enqueued: bool,
}

#[sqlx::test]
async fn every_job_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let mut cells = Vec::new();
    for &from in JOB.states {
        for &event in JOB.events {
            let expected = expect(from, event);
            let cell = match expected {
                Expect::Unreachable(_) => (None, false, Vec::new()),
                _ => run_cell(&app, from, event).await,
            };
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: cell.0,
                accepted: cell.1,
                broken: cell.2,
            });
        }
    }

    write_results(&JOB, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn run_cell(
    app: &App,
    from: JobState,
    event: JobEvent,
) -> (Option<JobState>, bool, Vec<&'static str>) {
    sqlx::query!("DELETE FROM jobs")
        .execute(&app.db)
        .await
        .unwrap();
    let mut bench = Bench {
        app: app.clone(),
        worker: Worker::new(app.clone(), vec![Handler::of::<Scripted>()]),
        enqueued: false,
    };
    bench.arrange(from).await;
    let runs_before = RUNS.load(Ordering::SeqCst);
    let accepted = bench.apply(event).await;
    let runs = RUNS.load(Ordering::SeqCst) - runs_before;
    let row = bench.row().await;
    let state = bench.state(row.as_ref());
    let mut broken = Vec::new();
    let polled = matches!(event, E::RunSucceeds | E::RunFails);
    let runnable = matches!(from, S::Queued | S::LastAttempt);
    if runs != usize::from(polled && runnable) {
        broken.push(RUNS_ONLY_WHEN_DUE);
    }
    if let Some(row) = &row {
        let settled_failure = event == E::RunFails && matches!(state, S::Waiting | S::Failed);
        if settled_failure && (!row.has_error || row.held) {
            broken.push(FAILURES_ARE_KEPT);
        }
        if row.attempts > row.max_attempts {
            broken.push(NEVER_PAST_ITS_ATTEMPTS);
        }
    }
    (Some(state), accepted, broken)
}

impl Bench {
    async fn arrange(&mut self, from: JobState) {
        match from {
            S::New => return,
            S::Queued => self.apply(E::Enqueue).await,
            S::LastAttempt => self.enqueue_last_attempt().await,
            S::Waiting => {
                self.apply(E::Enqueue).await;
                self.apply(E::RunFails).await
            }
            S::Running => {
                self.apply(E::Enqueue).await;
                self.apply(E::WorkerDies).await
            }
            S::RunningLastAttempt => {
                self.enqueue_last_attempt().await;
                self.apply(E::WorkerDies).await
            }
            S::OutOfAttempts => {
                self.enqueue_last_attempt().await;
                self.apply(E::WorkerDies).await;
                self.apply(E::LockExpires).await
            }
            S::Failed => {
                self.enqueue_last_attempt().await;
                self.apply(E::RunFails).await
            }
            S::Done => unreachable!("Done is terminal"),
        };
        let row = self.row().await;
        assert_eq!(
            self.state(row.as_ref()),
            from,
            "arranging {from:?}: {row:?}"
        );
    }

    async fn enqueue_last_attempt(&mut self) -> bool {
        self.apply(E::Enqueue).await;
        sqlx::query!("UPDATE jobs SET max_attempts = attempts + 1")
            .execute(&self.app.db)
            .await
            .unwrap();
        true
    }

    async fn apply(&mut self, event: JobEvent) -> bool {
        let db = &self.app.db;
        match event {
            E::Enqueue => {
                enqueue(&Scripted {}, db).await.unwrap();
                self.enqueued = true;
                true
            }
            E::RunSucceeds | E::RunFails => {
                SUCCEED.store(event == E::RunSucceeds, Ordering::SeqCst);
                self.worker.run_next().await.unwrap()
            }
            E::BackoffPasses => {
                sqlx::query!("UPDATE jobs SET run_at = now() - interval '1 second' WHERE status = 'queued' AND run_at > now()")
                    .execute(db)
                    .await
                    .unwrap()
                    .rows_affected()
                    > 0
            }
            E::WorkerDies => self.worker.claim_and_stop().await.unwrap(),
            E::LockExpires => {
                sqlx::query!("UPDATE jobs SET locked_until = now() - interval '1 second' WHERE locked_until > now()")
                    .execute(db)
                    .await
                    .unwrap()
                    .rows_affected()
                    > 0
            }
            E::FailedCleanup => {
                let later = App {
                    clock: Clock::Fixed(time::OffsetDateTime::now_utc() + Duration::days(31)),
                    ..self.app.clone()
                };
                delete_old_failed_jobs(&later).await.unwrap() > 0
            }
        }
    }

    async fn row(&self) -> Option<Row> {
        sqlx::query_as!(
            Row,
            r#"SELECT status = 'failed' AS "failed!",
                COALESCE(locked_until > now(), false) AS "held!",
                run_at > now() AS "later!",
                attempts, max_attempts,
                last_error IS NOT NULL AS "has_error!"
            FROM jobs"#
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap()
    }

    fn state(&self, row: Option<&Row>) -> JobState {
        let Some(row) = row else {
            return if self.enqueued { S::Done } else { S::New };
        };
        let last = row.attempts >= row.max_attempts;
        match row {
            Row { failed: true, .. } => S::Failed,
            Row { held: true, .. } if last => S::RunningLastAttempt,
            Row { held: true, .. } => S::Running,
            Row { later: true, .. } => S::Waiting,
            _ if last => S::OutOfAttempts,
            _ if row.attempts + 1 >= row.max_attempts => S::LastAttempt,
            _ => S::Queued,
        }
    }
}
