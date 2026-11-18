---
paths:
  - "src/action.rs"
  - "src/action/**"
  - "src/*/actions.rs"
  - "src/*/actions/**"
  - "src/permission.rs"
---

# Actions

The app is AI-native: every feature is one **action** that the page, the
JSON API and the MCP endpoint all call, so permission checks can't be
skipped by any path. New features follow the `new-action` skill.

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
- Owned rows are scoped by `caller.owner_id()` (teams later: `tenancy.md`).
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
- Every write records an account event: `account-events.md`. Ids: `public-ids.md`.
