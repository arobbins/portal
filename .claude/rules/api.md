---
paths:
  - "src/action.rs"
  - "src/api.rs"
  - "src/api/**"
  - "src/*/actions.rs"
  - "src/user/events.rs"
  - "src/secret.rs"
  - "src/api_tokens/**"
  - "src/profile/**"
  - "src/pages/api_docs.rs"
  - "src/pages/llms_txt.rs"
  - "e2e/tests/api-*.spec.ts"
  - "e2e/support/api.ts"
---

# Actions, API tokens and the JSON API

The app is AI-native: every feature is one **action** that the page, the
JSON API and the MCP endpoint all call, so permission checks can't be
skipped by any path. New features follow the `new-action` skill.

## Feature modules

`src/<feature>.rs` + `src/<feature>/`: `actions.rs`, `queries.rs`, `pages/`,
`emails.rs`. Features today: `auth` (sign-in, sign-up, emailed links, password,
session), `passkeys`, `profile`, `api_tokens`, `devices` (signed-in
browsers), `activity` (the account event log as a list), `admin` (the admin
area: users list and detail, `webserver admin` command), `account` (the
account shell: sections, pages, cards, nav, the sign-in and devices pages). `user/` is only the `User` model, its
queries, `name.rs` and the event log; `api/` is only HTTP plumbing (auth,
bodies, problems, scopes, catalog, `v1` routes).

## Actions (`src/action.rs`, `<feature>/actions.rs`)

- Signed-in actions: `async fn verb_thing(env: &Env<'_>, caller: &Caller, input: &Input)
  -> ActionResult<Output, Problem>` (`Result<Result<Output,
  ActionError<Problem>>>`: outer = server error, inner = denied or a user
  problem). Sync actions without I/O drop `env` and the outer `Result`.
  Actions never take `cx` or the session; pages pass `&env(cx)`.
- Public actions (before sign-in: `sign_in_with_password`, `register`,
  `request_password_reset`, `set_password_from_link`, passkey sign-in) take no
  `Caller` and return `Outcome<Output, Problem>`; they rate-limit themselves.
- **Authorization lives in the action**, first thing:
  `authorize!(caller.require(Permission::X))` (`src/permission.rs`). A
  `Permission` is checked twice: the caller's workspace role must grant it
  (`WorkspaceRole::Owner` grants everything today), and a token caller also
  needs the permission's `api_scope()` (with `implies`). Security-sensitive
  permissions (password, email, passkeys, API tokens, account deletion,
  administering users) have no API scope, so a token is always `Denied::BrowserOnly`. Add a permission
  for every new capability; never check scopes directly. Return problems
  with `return fail(Problem::X)`.
- Owned rows are scoped by `caller.owner_id()`. **Tenancy plan**: `Caller` has
  a `workspace` (`Workspace::Personal` today, so `owner_id()` is the user's
  id). Teams add `Workspace::Organization { id, role }`, roles beyond
  `Owner` in `WorkspaceRole::grants`, an `org_id` on owned tables and a
  membership check where `Caller` is built; actions don't change because
  they already ask for `owner_id()` and a `Permission`.
- Lists a page shows come from actions too (`list_passkeys`,
  `list_api_tokens`), so an API or MCP tool can expose them later.
- Pages build callers with `browser_caller(cx)` (procedures; `None` → return
  `FieldError::session_expired()`) or `require_browser_caller(cx)?` (POST
  routes; redirects to sign-in). `authenticate(cx, audience)` (→ `authenticate_header(env, header, audience)`) builds
  `Caller::api_token(..)` for the API. Pages turn errors into their problem
  with `error.for_browser()?` (a browser caller is never denied).
- Input structs derive `Deserialize` + `JsonSchema` with
  `#[serde(deny_unknown_fields)]` and `#[schemars(description = "…",
  length(..))]` on each field (attributes, not doc comments; limits from
  constants such as `MAX_NAME_LENGTH`). Choices are typed enums
  (`ApiScope`, `TokenLifetime`) with `#[serde(rename)]` values, so a bad
  value is rejected at the boundary (400 for a form, `invalid_json` for the
  API). The same struct is the POST route's `Form<…>`, the API body and the
  MCP arguments. Output structs derive `Serialize` + `JsonSchema` and are
  never a DB model; browser-only actions may hand the model back to the page
  (`change_password` returns the `User` to sign in again).
- Problems implement `ActionProblem` with one `spec()`:
  `ProblemSpec::new("snake_case_code", "Message.").on_field(FIELD)
  .with_status(StatusCode::X)` (code stable, field names the input, status
  default 422); wrapped problems return the inner `problem.spec()`.
  Shared problems are wrapped, never copied: `NameProblem`,
  `PasswordProblem`, `CurrentPasswordProblem` (from
  `check_current_password`, which also rate-limits and counts failures), and
  `TooManyAttempts`. Use `Infallible` when an action can't fail.
- Procedures return `Result<Result<T, FieldError>>`; `FieldError` is a
  `#[record]` (`field`, `message`) built with `FieldError::of(problem)`.
  HTML pages render `page_status(problem)` (200 for fixable problems, 429
  for rate limits); JSON keeps `problem.status()`.
- Every account change (including sign-up, email verification and
  answering the passkey prompt) records an `account_events` row with `record_event(env,
  &EventSource::of(caller) /* or ::browser(user_id) */, kind, json!({…}), &mut
  *tx)` in the same transaction. `detail` is a JSON object of public ids and
  names (never secrets). The caller's channel and token id are stored, which
  is what a future "what your apps did" page reads. Add an
  `AccountEventKind` (+ `ALL`) and extend the table's `kind` CHECK (new
  migration). Not events: usage stamps (`last_used_at` on passkeys and
  tokens) and system cleanup (expired links, abandoned sign-ups).

## Public ids (`src/secret.rs`)

Rows a browser or API can name (API tokens, passkeys) have a `public_id`
(`PublicIdKind::ApiToken` → `tok_…`, `Passkey` → `pk_…`, 20 base62
characters, DB format CHECK). Forms, URLs, events and API responses use it;
the integer `id` never leaves the server. Actions check the format with
`kind.is_valid(id)` before querying and always pair it with the owner.
`random_base62` and `hash_secret` are the only random-secret helpers.

## JSON API (`src/api/`)

- **Registry** (`api/registry.rs`): every API endpoint is an `ApiAction`
  impl next to its action (id, group, method, path, summary, description,
  scope, `Input`/`Output`/`Problem` types, examples, problem docs) listed
  in `endpoints()`. The docs page, `/api/v1/openapi.json`, `/llms.txt` and
  the MCP tools are generated from it; nothing about an endpoint is
  written twice.
- Routes live in `api/v1/<feature>.rs` as Topcoat `#[route]`s named
  `<verb>_<thing>_route` whose whole body is `serve::<A>(cx)` (no body) or
  `serve_with_body::<A>(cx, body)`; they authenticate, read the JSON body,
  run the action and turn server errors into 500 `internal_error`. Public
  endpoints use `public_api_call`. No logic in routes.
- Errors are RFC 9457 `application/problem+json`: `type`, `title`,
  `status`, `detail`, plus `code` and, for field problems, `errors: [{field,
  code, message}]` (`ApiProblem::of(problem)`; extra members with
  `.extra(key, value)`). The browser's own JSON routes (passkeys) use the same
  shape. Bad JSON/unknown fields → 400 `invalid_json`, wrong content type →
  415, bodies over 64 KB → 413 (routes with a body add
  `with(BodyLimit::max(MAX_JSON_BODY))`), unknown `/api/*` paths → 404
  `not_found`. A wrong method gets Topcoat's 405 with an `Allow` header.
  Write `detail` so a model knows how to fix the request.
- Every response is `Cache-Control: no-store`.
- Conventions for when they're first needed: lists use cursor pagination
  (`?cursor=&limit=`, response `{ "data": [...], "next_cursor": … }`);
  POSTs that create something accept an `Idempotency-Key` header; breaking
  changes go to `/api/v2`.

## MCP (`src/mcp.rs`, `POST /mcp`)

- Streamable HTTP, stateless JSON-RPC 2.0 (`initialize`, `ping`,
  `tools/list`, `tools/call`; notifications get 202). Every registered
  `ApiAction` is a tool named after its id (`get_profile`), with its request
  schema as `inputSchema` and response schema as `outputSchema`.
- Only tokens issued for `TokenAudience::Mcp` ("Used by: AI assistants" on
  the Developer tab) work here, and they don't work on `/api/v1`.
  `tools/list` shows only the tools the token's permissions allow.
- Tool problems are results with `isError: true` and the problem+json body
  as text (so the model can read `code` and `detail`); bad arguments and
  unknown tools are JSON-RPC `-32602` errors.

## Docs (`/docs/api`, `/api/v1/openapi.json`, `/llms.txt`)

- Both (and `/llms.txt`) are generated from the registry: `endpoint::<A>()`
  turns each `ApiAction` into an `Endpoint` (id `<verb>-<thing>`, summary
  "Verb your thing", `schema_for!` request/response, examples built from
  real values, the scope error, body errors for methods with a body, and the
  action's problems). Every error response in the OpenAPI document carries a
  real problem+json example per code. Field tables read the schemars
  descriptions and length limits, so write good `#[schemars(description)]`s.
  Never hand-write an error code or field list on the page.
- Adding an endpoint means an `ApiAction` impl plus a line in
  `registry::endpoints()`; `server/tests.rs` tests that every catalog entry exists and needs a token, and
  `api-docs.spec.ts` checks the page and the OpenAPI document.
- Error enums the docs list need `pub const ALL` (`ApiAuthProblem`,
  `BodyProblem`, `NameProblem`); shared API errors are `ApiFailure`.

## API tokens (`src/api_tokens/`)

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
