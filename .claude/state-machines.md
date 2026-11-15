# State machines

Every piece of state with more than two values is a Rust enum (or one signal
holding one name), and its transitions live in one place. When adding state,
extend one of these or add a row here.

| State | Type | Transitions |
|---|---|---|
| Account | `AccountStatus` (`AwaitingPassword` → `Active`) | `query_set_password_and_verify_user`; DB `CHECK ((password IS NULL) = (verified_at IS NULL))` |
| Signed in | `SessionAuth` in the session | `sign_in`; dropped by `load_signed_in_user` when the user or `session_version` changes; `logout` |
| Emailed link | `user_tokens` row per `TokenPurpose` | `email_user_link` (replaces) → `query_consume_user_token` (once) / expiry → `cleanup` |
| Flash | `Flash` (one slot) | `set_flash` → `take_flash` on a page that accepts it |
| Toast | one `Signal<String>` per page | name or `NO_TOAST`; click / animation end → `NO_TOAST` |
| Notice | one `stage` signal | `shown` → `dismissing` → `dismissed` |
| Form submit | `submitting` signal | idle → busy on submit → idle on bfcache restore (reload) |
| Form failure | `*Problem` enums | rendered after `redisplay(…)` |
| Theme | `Theme` + cookie | `POST /theme` or `window.setTheme` (`assets/js/theme.js`) |
| Account tab | `AccountTab` (`ALL`, `slug`, `path`, `sections`) + `AccountSection` (`anchor`, `label`, `path`) | URL → `from_slug`, unknown → 404 |
| Passkey prompt | `users.passkey_prompt_after` (NULL / date / `infinity`) | `answer_passkey_prompt` (`PromptAnswer::NotNow` / `Never`); due = no passkeys and not in the future |
| Passkey prompt grant | session `PasskeyPromptGrant { user_id, granted_at }` | `grant_passkey_prompt` (prompt shown) → `passkey_prompt_granted` (5 min) → `revoke_passkey_prompt` / `sign_in` |
| API token | `api_tokens` row (`ApiScope`s, `TokenAudience`, expiry) | create (password + email) → used (`last_used_at`) → revoked / password reset / expired → `cleanup` after 7 days |
| Caller | `Caller.via`: `Channel::Browser` / `ApiToken { id, scopes }`; `Caller.workspace`: `Workspace::Personal` | built per request (`browser_caller` / `authenticate`); checked by `authorize!(caller.require(Permission))` (role, then token scope) |
| Account event | `account_events` row (`AccountEventKind`, JSON `detail`, channel, token id) | `record_event` in every account change's transaction (append-only; usage stamps and cleanup aren't events) |
| Job | `jobs` row (a `trait Job` type: `SendEmail` / `PasswordResetJob` / `CleanupJob`, registered in `server::job_handlers()`; `queued` / `failed`) | `enqueue` → claimed (`SKIP LOCKED`, 5-min lock) → deleted on success / retried with backoff / `failed` after `max_attempts`; one `cleanup` job reschedules itself hourly |
| Rate limit | `rate_limit_hits` rows per bucket + key | `allow_*` (insert if under the limit) / `record_*` failures / `clear_failed_logins`; rows older than a day removed by cleanup |
| Permission | `Permission` + `WorkspaceRole` (`Owner`) | checked by `Caller::require`: role grants it, then token scope if the caller is a token |
| Config | `Config`, `Mailer`, `RateLimitMode` | parsed once at startup |
