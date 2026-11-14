CREATE TABLE jobs (
    id BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('send_email', 'send_password_reset', 'cleanup')),
    payload JSONB NOT NULL DEFAULT '{}'::JSONB CHECK (jsonb_typeof(payload) = 'object'),
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'failed')),
    run_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until TIMESTAMPTZ,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 5 CHECK (max_attempts > 0),
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_jobs_due ON jobs (run_at) WHERE status = 'queued';
CREATE UNIQUE INDEX idx_jobs_one_cleanup ON jobs (kind) WHERE kind = 'cleanup' AND status = 'queued';
