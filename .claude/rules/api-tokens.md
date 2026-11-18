---
paths:
  - "src/api_tokens/**"
  - "src/api_tokens.rs"
  - "src/api/auth.rs"
  - "src/api/scope.rs"
  - "e2e/tests/api-tokens.spec.ts"
  - "e2e/support/api.ts"
---

# API tokens

- **Format** (`api_tokens/token.rs`): `wsk_` + 43 random base62 characters +
  a 6-character checksum (first 4 bytes of SHA-256 of the secret, base62).
  `is_well_formed` rejects anything else before a database lookup. Only the
  SHA-256 hash and the last 4 characters (shown as `wsk_…abcd`) are stored;
  never argon2.
- **Scopes** (`api/scope.rs`): `ApiScope` with `implies` (write implies
  read); no wildcard scope. A scope with `requires_admin` is dropped at
  request time for non-admins. Adding a scope: enum + serde name, `ALL`,
  labels, and the `api_tokens.scopes` CHECK (new migration).
- **Audience**: `TokenAudience` (`Api`, `Mcp`; chosen when creating the
  token, default `Api`). `authenticate` rejects
  tokens issued for another audience.
- **Requests** (`api/auth.rs`): only `Authorization: Bearer`. Missing → 401
  `WWW-Authenticate: Bearer realm="api"`; unknown/expired/revoked/malformed
  all give the same 401 `invalid_token`; a missing scope → 403
  `insufficient_scope` with `scope="…"` in `WWW-Authenticate`. Limits: 20
  invalid tokens per IP per 10 min, and 600 calls per rolling minute per token,
  both 429 with `Retry-After`. `last_used_at` is written at most every 5
  minutes.
- **Creating/revoking** are browser-only actions (`api_tokens/actions/create.rs`, `manage.rs`):
  current password (a wrong one counts as a failed login), an email (subject
  to the per-address limit), an event, shown once. **Never put a token in a
  flash or the session** (sessions are stored in Postgres). Max 50 per user,
  expiry required. A password reset deletes all tokens (one `password_reset`
  event with `api_tokens_revoked`); a routine password change keeps them;
  `cleanup` deletes them 7 days after expiry.
- Tests: `api-tokens.spec.ts`, `api-profile.spec.ts` (`support/api.ts`
  helpers: `createToken(page, name, access)`, `bearer`, `getMe`), Rust tests
  in `action/`, `api/auth.rs`, `profile/actions.rs`,
  `api_tokens/actions/`, `api_tokens/queries.rs`; `api_tokens/testing.rs`
  has `issue_token` / `delete_token` for other features' tests.
