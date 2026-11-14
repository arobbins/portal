---
paths:
  - "src/**"
---

# Where things live

- `src/main.rs` — axum server: sessions, db pool, `/assets`, the Topcoat
  fallback, and the `webserver seed` subcommand.
- `src/config.rs` — `Config` (all settings, parsed and validated at startup),
  in Topcoat app context as `config(cx)`.
- `src/mail.rs` — `Mailer` (`postmark` | `log` | `file`), built by `Config`,
  read with `mailer(cx)`.
- `src/seed.rs` — `webserver seed`: creates/migrates/resets a `*_test` database
  and inserts the e2e users (refuses any other database name).
- `src/theme.rs` — `Theme` enum and `theme` cookie parsing.
- `src/app.rs` — Topcoat router builder, root `#[layout("/")]` (HTML document,
  header, footer, 404 error boundary), and `unknown_page`, a catch-all
  page that returns a 404 with its own title (written out instead of
  `not_found!`, which can't take `with(...)`).
- `src/layout/` — header, nav, footer, logo, page heading.
- `src/pages/` — general pages (`#[page("/path")]`); `src/pages/styleguide/` —
  one section per component.
- `src/action.rs` — `Caller` (browser or API token, `workspace`,
  `owner_id()`), `Denied`, `ActionError`, `ActionProblem`, `ActionResult` /
  `Outcome`, `fail`, `authorize!`, `FieldError`, `TooManyAttempts`,
  `page_status`, `browser_caller` / `require_browser_caller`.
- `src/auth/` — sign-in, sign-up, emailed links and passwords: `actions.rs`,
  `session.rs` (`current_user`, `require_user`, `sign_in`, passkey prompt
  grant), `password.rs` (`hash_password`, `password_matches`,
  `check_current_password`, `check_new_password`), `links.rs`
  (`TokenPurpose`, `email_user_link`), `queries.rs`, `emails.rs`, `pages/`.
- `src/passkeys/` — `actions.rs`, `routes.rs` (JSON), `webauthn.rs`,
  `queries.rs`, `emails.rs`, `pages/` (Security card, sign-in prompt).
- `src/profile/` — profile actions and the Profile tab.
- `src/api_tokens/` — token actions, format (`token.rs`), queries, emails and
  the Developer tab.
- `src/account/` — `tabs.rs` (`AccountTab`, `AccountSection`) and the account
  shell pages (layout, nav, menu, Security tab, placeholders).
- `src/user/` — `User` model, user queries, `name.rs`, `events.rs`
  (`record_event`, `EventSource`).
- `src/secret.rs` — `random_base62`, `hash_secret`, `PublicIdKind`.
- `src/web.rs` — `GlobalState`, `globals(cx)`, `session(cx)`, `db(cx)`,
  `inject_globals` axum middleware.
- `src/api/` — HTTP plumbing only: bearer auth (`auth.rs`), `api_call`
  wrappers (`handler.rs`), problem+json responses (`problem.rs`), JSON bodies
  (`body.rs`), scopes (`scope.rs`), routes in `v1/<feature>.rs`, and the
  endpoint catalog (`catalog.rs`) that generates the `/docs/api` page
  (`pages/api_docs.rs`) and `/api/v1/openapi.json`.
- `src/jobs.rs` — `Job`, `enqueue`, `Worker` (claims, retries, backoff).
- `src/permission.rs` — `Permission`, `WorkspaceRole`.
- `src/routes.rs` — named cross-feature links (`login_url`, `account_url`, …).
- `src/mcp.rs` — `POST /mcp` (JSON-RPC tools from the registry).
- `src/health.rs` — `/healthz`, `/readyz` (plain axum, outside sessions and
  the canonical-host redirect).
- `src/pages/llms_txt.rs` — `/llms.txt`.
- `src/rate_limit.rs` (Postgres sliding windows), `src/cleanup.rs` (the hourly cleanup job), `src/flash.rs`,
  `src/utils.rs`.
