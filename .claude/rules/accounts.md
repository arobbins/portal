---
paths:
  - "src/user/**"
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
  (`GET /user/verify/{token}`) only *shows* the set-password form; `POST` to
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
  `require_user(cx)` (sync, no extra query). Never cache user data in the
  session.
- **Account status** comes only from `user.status()` → `AccountStatus`
  (`AwaitingPassword` / `Active { password_hash }`). Never branch on
  `password` / `verified_at` directly. The only transition to Active is
  `query_set_password_and_verify_user`; the DB enforces
  `CHECK ((password IS NULL) = (verified_at IS NULL))`, so new users are
  always inserted without a password. `role` / `subscriber_status` are the
  `Role` / `SubscriberStatus` enums (select them with `role AS "role: Role"`;
  the DB has matching `CHECK`s).
- **Cleanup** (`src/cleanup.rs`, hourly): deletes expired `user_tokens`,
  expired sessions, and sign-ups that never set a password after 7 days and
  have no live link.
- **Untrusted input limits**: parse emails with `utils::parse_email` (trims,
  lowercases, rejects >254 chars or no `@`) before they reach rate limiters,
  queries or the DB; names are capped by `check_name`, passwords by
  `check_new_password`.
- **Checking a password**: always use `password_matches(hash, password)`;
  pass `None` when there's no usable account so it still does a full argon2
  check (against an unmatchable hash) and logins take the same time whether
  or not the email exists.
- **Rate limits** (`src/rate_limit.rs`, `governor`, in memory, read with
  `rate_limits(cx)`): password submissions (login + both set-password forms)
  10/min per IP, email requests (sign-ups + password resets) 5 per 10 min per
  IP, and at most 3 emails per hour to any one address. Failed logins are
  counted per account (`FailureWindow`, only wrong email/password counts, a
  successful login clears it): 5 per 15 min per account+IP, 50 per hour per
  account from any IP — then even the right password gets "Too many
  attempts". Per-IP blocks show a friendly message with status 429; the
  per-email limit silently skips the email (same response, so it reveals
  nothing). `RATE_LIMITS=relaxed` (used by the e2e server) multiplies limits
  by 1000. Client IPs come from Topcoat's `client_ip(cx)` (the connection
  address via `attach_remote_addr`); **behind a proxy/load balancer set
  `TRUSTED_PROXY_HOPS`** to the number of proxies, or every visitor shares
  the proxy's IP. Counters reset on restart and aren't shared across servers.
- **Emailed links** use the `user_tokens` table: one row per token with a
  `purpose`, only the SHA-256 hash stored, single use (consumed with
  `DELETE … RETURNING`), expiry per purpose. Send one with
  `links::email_user_link(db, mailer, user_id, purpose, |token| email)`: it
  replaces the user's older links for that purpose, commits *before* sending
  (never hold a DB connection across a network call), and deletes the new
  link if the email fails. Postmark calls time out after 10 s. Add a
  `TokenPurpose` variant for new flows (e.g. email change). Link pages share
  `expired_link` (400) and new passwords share `new_password_form` +
  `password::check_new_password`.
- **Account settings pages** (`account_profile.rs`, `account_security.rs`)
  show each setting as a card (`account__content-card`) with an action
  button on the right of its header that opens a `modal` with the form.
  Inside the modal, submit through a `#[procedure]` (errors render in the
  dialog, success closes it and shows a toast) and keep a plain POST route
  as the no-JS fallback (errors come back as an error toast). Return
  `(field, message)` on failure so the right input gets `aria-invalid`.
- **Change password** (Security → Password, procedure `save_new_password`,
  fallback `POST /account/password`): needs the current password (a wrong
  one counts as a failed login, so it's rate-limited and locks out like
  login), then `query_update_password` bumps `session_version` and this
  device is signed back in with the new version, so only *other* devices
  are signed out. The new-password inputs are the shared
  `new_password_fields`.
- **Password reset** (`reset_password.rs`): `/reset-password` always answers
  "if an account exists, we've sent a link" (active → 1-hour reset link;
  awaiting password → a fresh sign-up link; unknown → nothing). The email is
  sent from a background task so the response takes the same time whether
  or not the account exists. `GET /reset-password/{token}` only shows the
  form; `POST` (rate-limited, and it checks the link exists *before* running
  argon2) consumes the token, sets the password with `query_reset_password`
  (bumps `session_version`, so other devices are signed out), signs in and
  shows a toast on `/account`.
