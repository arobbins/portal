use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    New,
    Queued,
    LastAttempt,
    Waiting,
    Running,
    RunningLastAttempt,
    OutOfAttempts,
    Failed,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobEvent {
    Enqueue,
    RunSucceeds,
    RunFails,
    BackoffPasses,
    WorkerDies,
    LockExpires,
    FailedCleanup,
}

use JobEvent as E;
use JobState as S;

pub const JOB: Machine<JobState, JobEvent> = Machine {
    name: "Job",
    summary: "One background job, from being queued to finishing or failing for good, with workers that can die halfway.",
    states: &[
        S::New,
        S::Queued,
        S::LastAttempt,
        S::Waiting,
        S::Running,
        S::RunningLastAttempt,
        S::OutOfAttempts,
        S::Failed,
        S::Done,
    ],
    terminal: &[S::Done],
    events: &[
        E::Enqueue,
        E::RunSucceeds,
        E::RunFails,
        E::BackoffPasses,
        E::WorkerDies,
        E::LockExpires,
        E::FailedCleanup,
    ],
    stored_in: &[
        "jobs (row)",
        "jobs.status",
        "jobs.run_at",
        "jobs.locked_until",
    ],
    rules: &[
        RUNS_ONLY_WHEN_DUE,
        FAILURES_ARE_KEPT,
        NEVER_PAST_ITS_ATTEMPTS,
    ],
    writers: &[
        writer("src/jobs.rs:enqueue", "query_insert_job", &[E::Enqueue]),
        writer(
            "src/jobs/worker.rs:schedule_repeating_jobs",
            "query_schedule_if_missing",
            &[E::Enqueue],
        ),
        writer(
            "src/jobs/worker.rs:repeat",
            "query_schedule_if_missing",
            &[E::RunSucceeds, E::RunFails],
        ),
        writer(
            "src/jobs/worker.rs:run_next",
            "query_claim_job",
            &[E::RunSucceeds, E::RunFails, E::WorkerDies],
        ),
        writer(
            "src/jobs/worker.rs:fail_abandoned",
            "query_fail_abandoned_jobs",
            &[E::RunSucceeds, E::RunFails],
        ),
        writer(
            "src/jobs/worker.rs:settle",
            "query_delete_job",
            &[E::RunSucceeds],
        ),
        writer(
            "src/jobs/worker.rs:settle",
            "query_retry_job",
            &[E::RunFails],
        ),
        writer(
            "src/jobs/worker.rs:settle",
            "query_fail_job",
            &[E::RunFails],
        ),
        writer(
            "src/jobs.rs:delete_old_failed_jobs",
            "query_delete_old_failed_jobs",
            &[E::FailedCleanup],
        ),
    ],
};

pub const RUNS_ONLY_WHEN_DUE: &str = "The job's code runs only when a worker polls and the job is due with attempts left: never while waiting, held by another worker, failed or out of attempts.";
pub const FAILURES_ARE_KEPT: &str = "A failed attempt keeps its error and releases the lock; a failed job stays until cleanup removes it a month later.";
pub const NEVER_PAST_ITS_ATTEMPTS: &str =
    "A job is never handed to a worker more times than its max_attempts, even when workers die.";

const NO_JOB: &str = "There's no job yet.";
const GONE: &str = "The job is finished and gone.";
const SEPARATE: &str = "Enqueuing again makes a separate job.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [JobEvent],
) -> Writer<JobEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: JobState, event: JobEvent) -> Expect<JobState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::New, E::Enqueue) => Moves(S::Queued),
        (S::New, _) => Unreachable(NO_JOB),
        (S::Done, _) => Unreachable(GONE),
        (_, E::Enqueue) => Unreachable(SEPARATE),

        (S::Queued | S::LastAttempt, E::RunSucceeds) => Moves(S::Done),
        (S::Queued, E::RunFails) => Moves(S::Waiting),
        (S::LastAttempt, E::RunFails) => Moves(S::Failed),
        (S::Queued, E::WorkerDies) => Moves(S::Running),
        (S::LastAttempt, E::WorkerDies) => Moves(S::RunningLastAttempt),
        (S::Waiting, E::BackoffPasses) => Moves(S::Queued),
        (S::Running, E::LockExpires) => Moves(S::Queued),
        (S::RunningLastAttempt, E::LockExpires) => Moves(S::OutOfAttempts),
        (S::OutOfAttempts, E::RunSucceeds | E::RunFails) => Moves(S::Failed),
        (S::Failed, E::FailedCleanup) => Moves(S::Done),
        _ => Refused,
    }
}

impl Labeled for JobState {
    fn id(self) -> &'static str {
        match self {
            S::New => "new",
            S::Queued => "queued",
            S::LastAttempt => "last_attempt",
            S::Waiting => "waiting",
            S::Running => "running",
            S::RunningLastAttempt => "running_last_attempt",
            S::OutOfAttempts => "out_of_attempts",
            S::Failed => "failed",
            S::Done => "done",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::New => "Not queued",
            S::Queued => "Due",
            S::LastAttempt => "Due, last attempt",
            S::Waiting => "Waiting to retry",
            S::Running => "Held by a worker",
            S::RunningLastAttempt => "Held, last attempt",
            S::OutOfAttempts => "Due, no attempts left",
            S::Failed => "Failed",
            S::Done => "Done",
        }
    }
}

impl Labeled for JobEvent {
    fn id(self) -> &'static str {
        match self {
            E::Enqueue => "enqueue",
            E::RunSucceeds => "run_succeeds",
            E::RunFails => "run_fails",
            E::BackoffPasses => "backoff_passes",
            E::WorkerDies => "worker_dies",
            E::LockExpires => "lock_expires",
            E::FailedCleanup => "failed_cleanup",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Enqueue => "Enqueued",
            E::RunSucceeds => "A worker polls; the work succeeds",
            E::RunFails => "A worker polls; the work fails",
            E::BackoffPasses => "The retry delay passes",
            E::WorkerDies => "A worker claims it and dies",
            E::LockExpires => "The worker's lock runs out",
            E::FailedCleanup => "Cleanup, a month later",
        }
    }
}

#[cfg(test)]
mod tests;
