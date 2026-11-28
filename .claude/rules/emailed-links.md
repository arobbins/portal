---
paths:
  - "src/auth/**"
  - "e2e/tests/register.spec.ts"
  - "e2e/tests/verify-email.spec.ts"
  - "e2e/tests/reset-password.spec.ts"
  - "e2e/tests/change-email.spec.ts"
  - "e2e/tests/change-password.spec.ts"
  - "e2e/tests/delete-account.spec.ts"
---

# Registration, emailed links and account flows

- **Registration is email-first**: `/register` takes name + email (+ a hidden
  `website` honeypot) and always redirects to `/register` with the
  `RegistrationSent` flash, which the page shows as an inline "Check your
  inbox" notice naming the address (`auth/pages/link_sent.rs`; reset uses it
  too), whatever the email's state (new → user with
  `password = NULL` + link, deleted again if the email fails; awaiting
  password → fresh link; active → "you already have an account" email), so
  it never reveals whether an account exists. The link
  (`GET /verify-email/{token}`, built by `TokenPurpose::path`) only *shows*
  the set-password form; `POST` to
  the same URL consumes the token, sets the password, verifies and signs in.
  GETs never consume tokens (email scanners prefetch links). Users without a
  password can't log in.
- **Emailed links** use the `user_tokens` table: one row per token with a
  `purpose`, only the SHA-256 hash stored, single use (consumed with
  `DELETE … RETURNING`), expiry per purpose. Send one with
  `auth::links::email_user_link(app, NewLink { user_id, purpose, email: &|url| … })`:
  it replaces the user's older links for that purpose and enqueues the
  `SendEmail` job in the same transaction (the worker sends it, so no DB
  connection is held across a network call). Postmark calls time out after 10 s. Add a
  `TokenPurpose` variant for new flows (e.g. email change) with its `path`,
  `lifetime` and `lifetime_text` (emails and pages quote it; a test checks
  each path is a real page). Link pages share `expired_link` (400) and the
  `link_is_valid` / `set_password_from_link` actions; new passwords share
  `new_password_form` + `password::check_new_password`.
- **Account flows** (each is one action under `auth/actions/`; read the
  action and its `tests.rs` for the details). The invariants:
  - *Change password* (`change_password.rs`): current password required;
    bumps `session_version` and signs this device back in with the returned
    user, so only other devices are signed out.
  - *Password reset* (`password_reset.rs`, `link_password.rs`): the request
    is one job insert whatever the email (same answer and timing); the POST
    checks the link before argon2, bumps `session_version`, deletes every API
    token and records one `password_reset` event. With two-factor on, the
    same POST needs the code (`two-factor.md`).
  - *Change email* (`change_email.rs`, `confirm_email.rs`): re-auth with
    `check_reauth`; the request only enqueues `EmailChangeJob`, so the answer
    never reveals whether the new address has an account. The new address
    lives in the link's `payload` and nothing changes until the link's POST,
    which bumps `session_version` (every device signs out; the confirming
    browser signs back in if it was that user) and notifies the old address.
  - *Delete account* (`delete_account.rs`): `DELETE` typed plus
    `check_reauth`; immediate, one transaction (goodbye email + delete row,
    everything cascades, so it's logged with `tracing`, not an event).
  - Lock order: a transaction that consumes, replaces or deletes a user's
    links locks the `users` row first (`query_lock_user`, `FOR NO KEY
    UPDATE`, so link inserts' foreign-key checks don't wait on it, or an
    `UPDATE users` before touching `user_tokens`), then the link rows.
    Look a link's user up without locking, then lock, then consume.
  - A password change or reset deletes pending email-change and
    password-reset links. A verification link only sets a password on an
    account that has none (`password IS NULL`); otherwise it's expired.
