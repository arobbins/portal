---
paths:
  - "src/user/**"
  - "src/auth/**"
  - "src/admin/**"
  - "src/two_factor/**"
  - "src/passkeys/actions/**"
  - "src/utils.rs"
  - "src/seed.rs"
---

# Users: status, passwords and untrusted input

- **Account status** comes only from `user.status()` → `AccountStatus`
  (`AwaitingPassword` / `Active { password_hash }` / `Disabled {
  password_hash }`). Never branch on `password` / `verified_at` /
  `disabled_at` directly; a new variant makes the compiler list every place
  that must decide what it means. The only transition to Active is
  `query_set_password_and_verify_user`; the DB enforces
  `CHECK ((password IS NULL) = (verified_at IS NULL))`, so new users are
  always inserted without a password. `role` is the `Role` enum
  (`Subscriber` / `Admin`; select it with `role AS "role: Role"`; the DB has
  a matching `CHECK`). Users have a public id (`usr_…`, DB default).
- **Disabled accounts** (`users.disabled_at`, set only by an admin): a
  correct password or passkey gets `AccountDisabled` (403) instead of a
  session, API tokens are rejected as `invalid_token`, no sign-in or reset
  emails are sent, and disabling bumps `session_version`, deletes every
  device row and every emailed link. The link queries also ignore disabled
  users and `EmailChangeJob` / `PasswordResetJob` send nothing for them, so a
  job queued before disabling can't hand out a working link. Enabling
  restores sign-in, passkeys and API tokens.
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
