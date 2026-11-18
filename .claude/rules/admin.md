---
paths:
  - "src/admin/**"
  - "src/admin.rs"
  - "e2e/tests/admin.spec.ts"
  - "e2e/support/console.ts"
---

# Admin area

- **Admin area** (`src/admin/`, `/admin/users`): `Permission::AdministerUsers`
  is `admins_only()` (needs `Role::Admin`) and has no API scope. Non-admins
  get a 404. Every admin page calls `require_admin(cx)`, which also needs
  an unlock (`/admin/confirm`: password or passkey via `check_reauth`) used
  within the last 15 minutes (`ADMIN_IDLE_LIMIT`, sliding, bound to
  `session_version` and to `users.admin_granted_at`, which is set exactly
  while the role is admin, so revoking and granting again locks it; a locked GET is remembered and reopened after
  confirming). Actions (disable, enable, sign out everywhere, resend
  confirmation, delete) refuse the admin's own account and other admins
  (`AdminUserProblem`), lock the target row (`FOR UPDATE`) inside their
  transaction, are idempotent (enabling an active account changes nothing),
  and record the event on the *target* user with channel `admin` and
  `{"admin": "usr_…"}`. Resending a confirmation email uses the per-address
  email limit; an admin deletion sends `account_removed_email` (not the
  self-service "if you didn't do this" email). Roles change only from the
  terminal: `webserver admin grant|revoke <email>` (channel `console`).
  Every admin action and console role change also writes an `admin_log`
  row (`log_admin_action`, same transaction; `AdminLogAction`) holding the
  admin's and target's public id and email as text, with no foreign keys, so
  it survives deletions and is never cleaned up; `/admin/log` shows the last
  100 (`Server console` when there's no admin). A self-service deletion
  records no event, because events cascade with the row; it's logged with
  `tracing`. The e2e seed includes `admin@example.test`; e2e tests make
  their own admin with `grantAdmin(email)` (`support/console.ts`).
