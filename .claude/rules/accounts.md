---
paths:
  - "src/user/**"
  - "src/auth/**"
  - "src/account/**"
  - "src/web.rs"
  - "src/rate_limit.rs"
  - "src/cleanup.rs"
  - "src/seed.rs"
  - "src/utils.rs"
  - "src/flash.rs"
  - "migrations/**"
  - "e2e/tests/**"
---

# Accounts, sessions and emailed links

- **Registration is email-first**: `/register` takes name + email (+ a hidden
  `website` honeypot) and always redirects to `/register` with the
  `RegistrationSent` flash, whatever the email's state (new → user with
  `password = NULL` + link, deleted again if the email fails; awaiting
  password → fresh link; active → "you already have an account" email), so
  it never reveals whether an account exists. The link
  (`GET /verify-email/{token}`, built by `TokenPurpose::path`) only *shows*
  the set-password form; `POST` to
  the same URL consumes the token, sets the password, verifies and signs in.
  GETs never consume tokens (email scanners prefetch links). Users without a
  password can't log in.
- **Signed-in state**: the session stores one `SessionAuth { user_id,
  session_version }` value (never separate keys); a session whose version no
  longer matches the user's is flushed, so **any query that changes a
  password (or should log out every device) must do
  `session_version = session_version + 1`**. `inject_globals` loads the user
  once per request (skipped for `/assets`) into
  `GlobalState.current_user: Option<Arc<User>>`; a session pointing at a
  missing user is flushed. Read it with `current_user(cx)` /
  `require_user(cx)` (sync, no extra query), or as a `Caller` with
  `browser_caller(cx)` / `require_browser_caller(cx)`. Never cache user data in the
  session.
- **Account status** comes only from `user.status()` → `AccountStatus`
  (`AwaitingPassword` / `Active { password_hash }`). Never branch on
  `password` / `verified_at` directly. The only transition to Active is
  `query_set_password_and_verify_user`; the DB enforces
  `CHECK ((password IS NULL) = (verified_at IS NULL))`, so new users are
  always inserted without a password. `role` is the `Role` enum (select it
  with `role AS "role: Role"`; the DB has a matching `CHECK`).
- **Cleanup** (`src/cleanup.rs`, run hourly as one `cleanup` job, so once
  per cluster): deletes expired `user_tokens`, expired sessions, sign-ups
  that never set a password after 7 days and have no live link, API tokens
  7 days past expiry, rate-limit hits older than a day, account events older
  than 13 months (`EVENT_RETENTION`) and failed jobs older than 30 days.
- **Email is always a job** (`src/jobs.rs`): code never calls the mailer;
  it `enqueue`s `SendEmail(email)` (a `Job`, built with `EmailTemplate`) (inside the same transaction as the
  change it reports, where there is one) and a worker sends it with retries
  and backoff. A failed send never fails the request.
- **Untrusted input limits**: parse emails with `utils::parse_email` (trims,
  lowercases, rejects >254 chars or no `@`) before they reach rate limiters,
  queries or the DB; names are capped by `check_name`, passwords by
  `check_new_password`.
- **Checking a password** (`auth/password.rs`): always use
  `password_matches(hash, password)`; pass `None` when there's no usable
  account so it still does a full argon2 check (against an unmatchable hash)
  and logins take the same time whether or not the email exists. A signed-in
  user re-entering their password goes through `check_current_password(env,
  user, password)`, which applies the password limiter and the per-account
  lockout and returns `CurrentPasswordProblem`. Actions that also accept a
  passkey confirmation take `Option<PasskeyProof>` and call `check_reauth`
  (`auth/reauth.rs`, returns `ReauthProblem`: `Missing` or the password
  problem); see `passkeys.md`.
- **Rate limits** (`src/rate_limit.rs`, sliding windows in the
  `rate_limit_hits` table, so they hold across instances and restarts; read
  with `env.limits()`, every check is `async` and returns
  `Result<bool, sqlx::Error>`): password submissions (login + both set-password forms)
  10/min per IP, email requests (sign-ups + password resets) 5 per 10 min per
  IP, and at most 3 emails per hour to any one address. Failed logins are
  counted per account (only wrong email/password counts, a
  successful login clears it): 5 per 15 min per account+IP, 50 per hour per
  account from any IP — then even the right password gets "Too many
  attempts". Per-IP blocks show a friendly message with status 429; the
  per-email limit silently skips the email (same response, so it reveals
  nothing). `RATE_LIMITS=relaxed` (used by the e2e server) multiplies limits
  by 1000. Client IPs come from Topcoat's `client_ip(cx)` (the connection
  address via `attach_remote_addr`); **behind a proxy/load balancer set
  `TRUSTED_PROXY_HOPS`** to the number of proxies, or every visitor shares
  the proxy's IP.
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
- **Account settings** (`account/`): tabs are `AccountTab`, the anchored
  cards inside them are `AccountSection` (`label()`, `anchor()`, `path()`); the
  nav, card ids and redirects all come from those two enums. Each tab is a
  `<feature>/pages/<name>_tab.rs` component (`profile_tab`, `security_tab`,
  `developer_tab`). Each setting is a card with an action button on the
  right of its header that opens a `modal` with the form.
  Inside the modal, submit through a `#[procedure]` (errors render in the
  dialog, success closes it and shows a toast) and keep a plain POST route
  as the no-JS fallback (errors come back as an error toast). Procedures
  return a `FieldError` (`field`, `message`) so the right input gets
  `aria-invalid`.
- **Account flows** (each is one action under `auth/actions/`; read the
  action and its `tests.rs` for the details). The invariants:
  - *Change password* (`change_password.rs`): current password required;
    bumps `session_version` and signs this device back in with the returned
    user, so only other devices are signed out.
  - *Password reset* (`reset_password.rs`, `link_password.rs`): the request
    is one job insert whatever the email (same answer and timing); the POST
    checks the link before argon2, bumps `session_version`, deletes every API
    token and records one `password_reset` event.
  - *Change email* (`change_email.rs`, `confirm_email.rs`): re-auth with
    `check_reauth`; the request only enqueues `EmailChangeJob`, so the answer
    never reveals whether the new address has an account. The new address
    lives in the link's `payload` and nothing changes until the link's POST,
    which bumps `session_version` (every device signs out; the confirming
    browser signs back in if it was that user) and notifies the old address.
  - *Delete account* (`delete_account.rs`): `DELETE` typed plus
    `check_reauth`; immediate, one transaction (goodbye email + delete row,
    everything cascades, so it's logged with `tracing`, not an event).
  - A password change or reset deletes pending email-change links.
