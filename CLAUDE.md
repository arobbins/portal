# Webserver Project

Rust web app: axum + [Topcoat](https://github.com/tokio-rs/topcoat) 0.12.1. Axum
owns the server (sessions, middleware, `/assets`), and hands every other
request to a Topcoat router through `TowerService` as its fallback. Topcoat renders all pages on the server; interactivity comes from
small `$(...)` runtime expressions that Topcoat compiles to JavaScript (no
WASM). Postgres via sqlx (with migrations), tower-sessions, argon2 auth. CSS is
bundled/minified by Lightning CSS in `build.rs` (`assets/css/` →
`assets/dist/app.css`, served from `/assets`).

Detailed rules for each area live in `.claude/rules/` and load automatically
when you work on matching files: `accounts.md` (registration, sessions,
links, rate limits, password reset), `passkeys.md`, `api.md` (actions, API
tokens, bearer auth, scopes, JSON errors), `forms.md`,
`styling.md` (layout, CSS, theming), `server.md` (config, headers, email,
logging), `e2e.md` (Playwright conventions), `topcoat.md` (Topcoat
conventions and gotchas), `layout.md` (file map), `components.md`
(component catalog).

## Architecture principles

- **AI-native: every feature is one action** (`<feature>/actions.rs`) that
  the page, the JSON API (`/api/v1`) and the MCP endpoint (`/mcp`) all call — sign-in,
  sign-up and password reset included (public actions, no `Caller`). Pages,
  procedures and routes only call actions; permission checks live in the
  action (`authorize!(caller.require(Permission::X))`), and every
  write records an account event. New features ship to the UI and the API
  together: the `new-action` skill. Contract details: `.claude/rules/api.md`.
- **Feature modules**: `src/<feature>/` holds `actions.rs`, `queries.rs`,
  `pages/` and `emails.rs` (`auth`, `passkeys`, `profile`, `api_tokens`,
  `account`). `user/` is only the user model, its queries, names and the event
  log; `api/` is only HTTP plumbing.
- **One error contract**: problems are `ActionProblem` enums with a stable
  `code`, `message`, `field` and `status`. Procedures return `FieldError`,
  JSON (API and the browser's passkey routes) returns problem+json, HTML
  pages render the message with `page_status(problem)`.
- **No typed app paths**: links and redirects use `href!(handler)`,
  `AccountTab::X.path()` / `AccountSection::X.path()`, the path constants in
  `src/routes.rs` (`LOGIN_PATH`, `ACCOUNT_PATH`, …), or
  `TokenPurpose::path` for emailed links; `scripts/check_conventions.py`
  (in `./check.sh`) fails on a hard-coded path or a limit written into a
  message without a `*_matches_the_limit` test. Account POST routes are
  `/account/<thing>/<verb>`; signed-out forms post to their page's URL.
- **Features own their tables**: only a feature's own module calls its
  `query_*` functions; other features call a function it exposes
  (`api_tokens::lifecycle::revoke_all_for_user`). Links across features go
  through `src/routes.rs` (`LOGIN_PATH`, `ACCOUNT_PATH`, …), so core
  code never imports a page.
- **Slow or fallible work is a job** (`src/jobs.rs`): emails and cleanup
  are rows in `jobs`, run by workers in the web process (`JOB_WORKERS`,
  default 2) or a separate `webserver work`. State that must hold across
  instances (rate limits, jobs) lives in Postgres, never in memory.
- **Public ids**: rows a browser or API can name use a prefixed public id
  (`tok_…`, `pk_…`, `secret::PublicIdKind`); integer ids stay internal.
- **Compile-time safety first**: components, props, routes (`href!`), and sqlx
  queries are all checked by the compiler. Component variants are Rust enums
  (`ButtonSize::Sm`), never strings. Path constants (`routes.rs`,
  `AccountTab::path()`) are plain strings, so `server/tests.rs`
  (`every_named_path_is_a_real_page`) checks each one is a real route.
- **Native platform over JS**: use native HTML/CSS features where they exist —
  `<dialog>` + `command`/`commandfor` for modals, `popover`, CSS transitions and
  animations (the toaster auto-hides via a CSS animation + `@animationend`).
- **State lives at the lowest rung that works**: server data → render it in the
  page; widget state → a `signal(cx, ...)` inside the component; shared widget
  state → create the signal in the common parent and pass `&Signal<T>` down
  (see the styleguide toaster section). Prefer one signal holding *which*
  state you're in over several booleans that must agree: toasts use one
  `toast: Signal<String>` per page holding the visible toast's name (or
  `NO_TOAST`), and `toaster(toast: &toast, name: …, …)` shows itself when it
  matches, so only one toast can ever be open. Success toasts are flashes:
  `flash_toasts(toast: &toast, flashes: FLASHES)` renders one per accepted
  `Flash` (name + message come from `Flash::name()`/`message()`), and JS can
  show the same toast with `Flash::X.name()`. Other toast names are per-page
  `const`s, never literals. Errors inside a `<dialog>` go in the dialog, not
  a toast (an open modal covers toasts).
- **Forms are plain HTML POSTs**: a `#[route(POST ...)]` handler either
  `see_other(...)` redirects on success or returns
  `redisplay(path, failure)` (`web/context.rs`), which re-renders the GET page
  with a request-context value carrying the error (see `auth/pages/login.rs`).
  They work without JS; `@submit` only adds the busy state. Busy state:
  pass `submitting: Some(&is_submitting)` to `form(…)` and give the button
  `busy: $(is_submitting.get())` only. Controls, layout and errors: see
  `.claude/rules/forms.md`.
- **Success messages after a redirect are session flashes** (`src/flash.rs`):
  `set_flash(&session, Flash::ProfileSaved)` before `see_other(...)`, and the
  target page calls `take_flash(cx, FLASHES)` (a page `const` listing what it
  accepts), which removes it only if it's one the page accepts. Never put
  message flags in the URL (they reappear on refresh and can be faked). Add a
  `Flash` variant (with its `name()` and `message()`) for new messages.
- **Failure reasons are enums** implementing `ActionProblem`
  (`SignInProblem`, `RegisterProblem`, `ChangePasswordProblem`,
  `CreateTokenProblem`, …); render `(page_status(problem))` and
  `(problem.message())`, never compare message text. Shared problems are
  wrapped, never copied: `NameProblem` (`user/name.rs`), `PasswordProblem`
  and `CurrentPasswordProblem` (`auth/password.rs`), `TooManyAttempts`
  (`action.rs`).
- **Every sqlx query is its own `async fn query_<verb>_<thing>`** in the
  feature's `queries.rs`, returning `Result<T, sqlx::Error>`, with `db: impl
  PgExecutor<'_>` as the last parameter, ids by value and a param struct for
  multi-column inserts (`NewUser<'a>`); never inline sqlx in a handler, page
  or procedure. Recipe: the `new-query` skill.
- **Configuration is parsed once** (`src/config.rs`) and read with
  `config(cx)` / `app.url(path)`; nothing else reads env vars (except
  `DATABASE_URL`, read by `server::database_url()`).
- **Styling**: BEM classes with a prefix (`a-` component, `l-` layout, `p-`
  page, `u-` utility) are the contract (CSS targets only them, never element
  structure or ids); colors are role-named `light-dark()` variables built
  from `--palette-*` in `variables.css`, never hard-coded; sizes come from
  the `--space-*`, `--text-*`, `--radius-*` and `--duration-*` scales;
  status colour is the `Tone` enum; no inline styles; one breakpoint
  (`max-width: 760px`, plus 1100px inside `l-docs`), check pages at 390px.
  Details in `.claude/rules/styling.md`.
- **Wording**: sentence case everywhere; buttons are "Verb noun" and a
  modal's heading repeats the button that opens it ("Edit profile",
  "Change password", "Add passkey", "Create API token"); "Sign in" /
  "Sign out" / "Register"; passkeys are *removed*, API tokens are *revoked*
  (they're credentials handed to other apps). Messages are full sentences;
  "Please …" only when asking the visitor to do something.

## Clean-code rules (enforced by `scripts/clean_code.py`)

- **Dependencies are injected, never looked up**: `App { db, config,
  limits, webauthn, clock }` (`src/env.rs`) is built once in `server.rs`
  and put in Topcoat context; pages and routes read it with `web::app(cx)`,
  `web::config(cx)` or `web::env(cx)` (= `app.env(client_ip)`). Actions
  take `env: &Env` (`db()`, `limits()`, `now()`, `ip`) and never `cx` or the
  session (the passkey ceremony uses the `PasskeySession` trait); jobs get
  `&App`. Time comes only from `env.now()` / `app.clock.now()` (`Clock::Fixed`
  in tests); tests build `App::for_tests(db)` / `Config::for_tests()`.
- **Infrastructure knows no feature**: `action`, `env`, `jobs`, `mail`,
  `rate_limit`, `web` never import `auth`/`passkeys`/`profile`/`api_tokens`/
  `account`. Features plug in at the composition root: jobs implement
  `trait Job` and are listed in `server::job_handlers()`, cleanup calls each
  feature's own `delete_*` function. The one exemption is
  `web/middleware.rs` (`inject_globals` loads the signed-in user through
  `auth::session`), listed in `COMPOSITION` in `scripts/clean_code.py`.
- **Small units**: plain functions ≤ 40 lines, components/pages ≤ 80, files
  ≤ 300 lines before their tests. Split along responsibilities: a tab is a
  composition file plus `*_dialog.rs`, `*_fields.rs`, `*_routes.rs`
  (procedure + POST fallback), `*_view.rs` (view models such as
  `TokenRow::new`). Modal forms use `DialogForm` + `dialog_input`,
  `dialog_form_error`, `dialog_error_toast` (`components/form/dialog.rs`).
  Emails are built with `mail::EmailTemplate`, never `format!`.
- **Encapsulation**: feature `queries` modules are private; visibility is
  the narrowest that compiles (`pub(super)` inside `pages/`). Test helpers
  live in `#[cfg(test)]` code or a `testing.rs`; never `#[allow(dead_code)]`.
- The `clean-code-audit` skill re-runs the whole audit, scores it and
  tightens the limits; run it after a large feature.

## Security invariants (never break these)

- Any query that changes a password, or should sign out every device, sets
  `session_version = session_version + 1`.
- Account state comes only from `user.status()` → `AccountStatus`; never
  branch on `password` / `verified_at`. New users are inserted without a
  password.
- Check passwords only with `password_matches(hash, password)` (`None` when
  there's no usable account, so timing doesn't reveal it).
- Parse emails with `utils::parse_email` before they reach rate limiters,
  queries or the DB. Responses never reveal whether an account exists.
- Emailed-link GETs never consume tokens; tokens are hashed and single use.
- Every value from the browser (signals, procedure/shard args) is user
  input; procedures and shards bypass layout/page guards, so re-check auth.
- Never hold a DB connection across a network call.
- API tokens: only hashes are stored, they're accepted only from the
  `Authorization: Bearer` header and only for their own audience, they're
  never put in a flash, session, URL or log, and no token can change
  security settings (password, email, passkeys, tokens, account deletion).

## Layout

Feature modules are described under "Feature modules" above. The full file
map is `.claude/rules/layout.md` and the component catalog is
`.claude/rules/components.md`; both load when you open matching files. Use
existing components before writing page-specific markup.

## State machines

Every piece of state with more than two values is a Rust enum (or one signal
holding one name), and its transitions live in one place:

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

When adding state, extend one of these or add a row here.

## Topcoat

Conventions and gotchas are in `.claude/rules/topcoat.md` (loaded for any
`.rs` file). The ones that bite first: every `#[component]`/`#[page]`/
`#[route]`/`#[procedure]` takes `cx: &Cx` first; links are `href!(handler)`;
run `topcoat fmt` instead of hand-wrapping macros; a procedure call
*rejects* on network/server errors, so wrap it in the `raw!(… .catch(…))`
pattern.

## Development

From `webserver/`:

```sh
topcoat dev
```

Builds the app, bundles Topcoat's browser runtime, starts the server
(respects `HOST`/`PORT`, default `127.0.0.1:3000`) and live-updates the page on
changes (including `assets/css`). CLI:
`cargo install topcoat-cli --version 0.12.1` (it uses the crate's own
lockfile).

**Toolchain**: Rust is pinned in `rust-toolchain.toml` (rustup installs it
automatically); to upgrade, change it there, restart `topcoat dev`, run
`cargo clean` once (old build files aren't reused) and `./check.sh`.
`.cargo/config.toml` sets `global-min-publish-age = "3 days"`: Cargo won't
pick a crate version newer than that (a hijacked release is usually caught
within days). `Cargo.lock` versions are unaffected; to take a fresh fix
sooner, run that one update with
`CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow`.

**Before calling work done, run `./check.sh`** (through the `check` skill,
which prints only failures): `cargo fmt --check`,
`topcoat fmt --check`, `scripts/check_conventions.py`,
`cargo clippy --all-targets -- -D warnings`,
`cargo test`, and the Playwright suite. Format with
`topcoat fmt --rustfmt $(find src -name "*.rs")` (formats Rust and `view!`
bodies; never use `leptosfmt`). While iterating, use the `verify` skill
(one spec, Chromium only, plus screenshots).

**Migrations**: add a *new* file in `migrations/` (never edit an applied one;
a hook blocks it). Try it on the e2e test database first (any Playwright
run migrates it, and it holds older rows than dev), then `sqlx migrate run`
against the dev database (sqlx macros check queries against it at compile
time); recipe in the `new-query` skill. Query metadata for offline
builds lives in `.sqlx/` (committed): after changing a query run
`cargo sqlx prepare -- --all-targets` (the stop hook does it for you;
`./check.sh` fails if it's stale). Build without Postgres with
`SQLX_OFFLINE=true`.

**Behavior tests**: Playwright in `e2e/` (Chromium + Firefox + WebKit) on its
own server at :3100 with its own database and mail dir;
`cd e2e && npm test`. Conventions in `.claude/rules/e2e.md`.

**Screenshots**: `cd e2e && SHOT_PATHS=/login npm run screenshot` captures
pages in both themes at phone and desktop width on the e2e server
(`SHOT_SIGNED_IN=1` for account pages, `SHOT_VARIANTS` to compare CSS
options). Settings are in the `verify` skill.

**Claude Code setup** (`.claude/`):
- Hooks: every edited `.rs` file is formatted with `topcoat fmt --rustfmt`;
  before a turn ends, `scripts/check_conventions.py` and then
  `cargo clippy --all-targets -D warnings` run (clippy in
  `target/claude-check`, skipped when nothing changed); edits to applied
  migrations and generated files (`assets/dist/`, `target/`, `e2e/.mail`,
  `e2e/test-results/`) are blocked (reading them is fine, e.g. failure
  screenshots), and so is reading `.env`; session start reports branch, database,
  pending migrations and whether `topcoat dev` is running.
- Skills: `check` (full `./check.sh`, failures only), `clean-code-audit` (repeatable audit + ratchet), `new-action`, `new-component`, `new-query`, `new-flash`, `new-form-flow`,
  `new-e2e-test`, `verify`.
- After a non-trivial change, run the `convention-reviewer` agent on the diff
  and fix its must-fix findings before reporting done.

**Improving the AI workflow**: this setup should keep getting better from real
friction. While working, notice when something about the workflow itself
cost time or caused a mistake. For example:
- you did the same multi-file steps by hand for the second time (→ a skill);
- you broke a rule that a script could have caught (→ a hook or a check in
  `./check.sh`);
- a rule here was missing, wrong, stale, or loaded when irrelevant (→ edit
  CLAUDE.md or move it into `.claude/rules/`);
- a tool, hook or permission prompt kept getting in the way (→ settings);
- the reviewer missed something, or flagged something that's fine (→ edit
  `convention-reviewer`).

Don't change the setup silently mid-task. At the end of the task, add one
short "Workflow suggestion" line: what happened, the proposed fix, and the
file it would change. Only suggest it when it would clearly pay off again,
and do it once the user agrees.
