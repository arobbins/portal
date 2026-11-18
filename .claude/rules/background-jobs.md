---
paths:
  - "src/cleanup.rs"
  - "src/jobs.rs"
  - "src/jobs/**"
  - "src/mail.rs"
  - "src/mail/**"
  - "src/*/emails.rs"
---

# Cleanup and email jobs

- **Cleanup** (`src/cleanup.rs`, run hourly as one `cleanup` job, so once
  per cluster): deletes expired `user_tokens`, expired sessions, sign-ups
  that never set a password after 7 days and have no live link, API tokens
  7 days past expiry, rate-limit hits older than a day, account events older
  than 13 months (`EVENT_RETENTION`), failed jobs older than 30 days and
  device rows idle for a day or on an old `session_version`.
- **Email is always a job** (`src/jobs.rs`): code never calls the mailer;
  it `enqueue`s `SendEmail(email)` (a `Job`, built with `EmailTemplate`) (inside the same transaction as the
  change it reports, where there is one) and a worker sends it with retries
  and backoff. A failed send never fails the request.
