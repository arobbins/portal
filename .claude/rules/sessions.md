---
paths:
  - "src/auth/session.rs"
  - "src/auth/actions/sign_in.rs"
  - "src/devices/**"
  - "src/activity/**"
  - "src/web.rs"
  - "src/web/**"
  - "src/user/event*.rs"
  - "e2e/tests/devices.spec.ts"
  - "e2e/tests/security.spec.ts"
  - "e2e/tests/auth.spec.ts"
---

# Sessions, devices and activity

- **Signed-in state**: the session stores one `SessionAuth { user_id,
  session_version, device }` value (never separate keys) and every signed-in
  browser has a `user_devices` row (`devices/`, `dev_…` public id, user agent,
  IP, `last_seen_at` written at most every 5 minutes). A session is valid only
  while its device row exists and both versions match the user's, so **any
  query that changes a password (or should log out every device) must do
  `session_version = session_version + 1`**. Sign in only through
  `auth::session::sign_in(cx, session, user, SignInMethod)` (records a
  `signed_in` event; when a different account was signed in on this
  session, its session data is cleared first); after a version bump on *this* device use
  `stay_signed_in(app, session, user)`; sign out with `sign_out`.
  `inject_globals` loads the user once per request (skipped for `/assets`)
  into `GlobalState.current_user: Option<Arc<User>>`. Read it with
  `current_user(cx)` / `require_user(cx)` (sync, no extra query), or as a
  `Caller` with `browser_caller(cx)` / `require_browser_caller(cx)`. Never
  cache user data in the session.
- **Devices and activity** (Security tab, browser-only `ManageDevices` /
  `ViewActivity`): `devices/actions.rs` lists this user's live devices, signs
  one out (deletes its row; never the current one) or signs out every other
  device (bumps `session_version`, keeps this row); `activity/actions.rs`
  shows the latest 20 `account_events` as short neutral phrases ("Password
  changed", never "your"), because the same list appears on the admin
  user page (`summarize`, one per `AccountEventKind`: a new kind needs a
  summary, a test enforces it).
