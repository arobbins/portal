---
paths:
  - "src/**"
---

# Where things live

- `src/main.rs` — module list, the `Command` enum (`serve`, `work`,
  `seed`) and tracing setup.
- `src/server.rs` — the composition root: loads config, builds `App`,
  registers job handlers (`job_handlers()`), starts workers and the
  listener; `server/router.rs` layers axum (sessions, middleware, assets,
  health, the Topcoat fallback); `server/tests.rs` holds the integration
  tests (every catalog endpoint, every named path is a real page).
- `src/env.rs` — `Clock`, `App` (db, config, limits, webauthn, clock;
  `url()`, `env(ip)`, `for_tests`) and `Env` (what actions receive).
- `src/config.rs` — `Config` (all settings, parsed and validated at startup),
  read with `web::config(cx)` or `app.config`.
- `src/mail.rs` — `Mailer` (`postmark` | `log` | `file`), `MailError`, the
  `SendEmail` job; `mail/template.rs` — `EmailTemplate` + `Part` (text and
  escaped HTML from one description).
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
- `src/action.rs` + `src/action/` (`caller.rs`, `problem.rs` with
  `ProblemSpec`, `field_error.rs`, `browser.rs`) — `Caller` (browser or API token, `workspace`,
  `owner_id()`), `Denied`, `ActionError`, `ActionProblem`, `ActionResult` /
  `Outcome`, `fail`, `authorize!`, `FieldError`, `TooManyAttempts`,
  `page_status`, `browser_caller` / `require_browser_caller`.
- `src/auth/` — sign-in, sign-up, emailed links and passwords: `actions/`
  (one file per use case),
  `session.rs` (`current_user`, `require_user`, `sign_in`, passkey prompt
  grant), `password.rs` (`hash_password`, `password_matches`,
  `check_current_password`, `check_new_password`), `links.rs`
  (`TokenPurpose`, `email_user_link`), `queries.rs`, `emails.rs`, `pages/`.
- `src/passkeys/` — `actions/` (`registration`, `sign_in`, `manage`,
  `prompt`), `session.rs` (`PasskeySession` trait), `routes.rs` (JSON), `webauthn.rs`,
  `queries.rs`, `emails.rs`, `pages/` (Security card, sign-in prompt).
- `src/profile/` — profile actions, `api.rs` (its `ApiAction`s) and the Profile tab.
- `src/api_tokens/` — `actions/` (`create.rs`, `manage.rs`), `lifecycle.rs`
  (`find_grant`, `record_use`, `revoke_all_for_user`, cleanup), `lifetime.rs`,
  format (`token.rs`), private queries (typed `ApiToken`), emails, and
  `pages/` (`developer_tab`, `create_token_dialog`, `token_fields`,
  `token_list`, `token_routes`, `token_view`).
- `src/account/` — `tabs.rs` (`AccountTab`, `AccountSection`) and the account
  shell pages (layout, nav, menu, Security tab, placeholders).
- `src/user/` — `User` model, user queries, `name.rs`, `events.rs`
  (`record_event`, `EventSource`).
- `src/secret.rs` — `random_base62`, `hash_secret`, `PublicIdKind`.
- `src/web.rs` — `web/context.rs` (`globals`, `session`, `app`, `env`,
  `config`, `redisplay`) and `web/middleware.rs` (`inject_globals`).
- `src/api/` — HTTP plumbing only: bearer auth (`auth.rs`), `api_call`
  wrappers (`handler.rs`), problem+json responses (`problem.rs`), JSON bodies
  (`body.rs`), scopes (`scope.rs`), routes in `v1/<feature>.rs`, and the
  endpoint catalog (`catalog.rs`) that generates the `/docs/api` page
  (`pages/api_docs.rs`) and `/api/v1/openapi.json`.
- `src/jobs.rs` — `trait Job`, `JobError`, `enqueue`, `Handler`;
  `jobs/queue.rs` (queries), `jobs/worker.rs` (`Worker`: claims, retries,
  backoff, repeating jobs).
- `src/permission.rs` — `Permission`, `WorkspaceRole`.
- `src/routes.rs` — cross-feature path constants (`LOGIN_PATH`, `LOGOUT_PATH`,
  `REGISTER_PATH`, `ACCOUNT_PATH`, `API_DOCS_PATH`; test-only `PAGE_PATHS`).
- `src/mcp.rs` — `POST /mcp` (JSON-RPC tools from the registry).
- `src/health.rs` — `/healthz`, `/readyz` (plain axum, outside sessions and
  the canonical-host redirect).
- `src/pages/llms_txt.rs` — `/llms.txt`.
- `src/rate_limit.rs` (Postgres sliding windows), `src/cleanup.rs` (the hourly cleanup job), `src/flash.rs`,
  `src/utils.rs`.
