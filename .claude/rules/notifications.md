---
paths:
  - "src/preferences/**"
  - "src/devices/known.rs"
  - "src/devices/known/**"
  - "src/auth/session.rs"
  - "src/*/emails.rs"
  - "e2e/tests/notifications.spec.ts"
---

# Notifications and sign-in alerts

- **Notifications** (`preferences/notifications.rs`): security emails
  (password changed or reset, email changed, passkey added, API token
  created, two-factor on or off) are always sent. Optional emails have a
  `NotificationTopic` (today only `SignInAlerts`), are on until the user
  opts out (`notification_opt_outs`), check `wants(user_id, topic, db)`
  before queueing, and end with a link to the Notifications card. **New
  sign-in alerts**: `sign_in` sets an HttpOnly `known_device` cookie (only
  its hash is stored, per user, in `known_devices`); a password or passkey
  sign-in from a browser not known for that user queues "New sign-in to
  your account" (subject to the per-address email limit). Sign-ins from an
  emailed link (new account, password reset) mark the browser known
  without an alert. Response cookies are set through
  `web::set_response_cookie(cx, …)`, which `inject_globals` appends.
