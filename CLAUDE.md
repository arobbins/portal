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
links, rate limits, password reset), `passkeys.md`, `forms.md`,
`styling.md` (layout, CSS, theming), `server.md` (config, headers, email,
logging), `e2e.md` (Playwright conventions).

## Architecture principles

- **Compile-time safety first**: components, props, routes (`href!`), and sqlx
  queries are all checked by the compiler. Component variants are Rust enums
  (`ButtonSize::Sm`), never strings.
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
  `see_other(...)` redirects on success or `rewrite(...)`s back to the GET page
  with a request-context value carrying the error (see `user/pages/login.rs`).
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
- **Failure reasons are enums** with `message()` and, where it matters,
  `status()` (`LoginProblem`, `RegisterProblem`, `ResetRequestProblem`,
  `NameProblem`, `PasswordProblem`); render `(problem.status())` and
  `(problem.message())`, never compare message text. Shared rules live in
  `user/name.rs` and `user/password.rs`.
- **Every sqlx query is its own `async fn query_<verb>_<thing>`** returning
  `Result<T, sqlx::Error>`, with `db` as the last parameter and a param struct
  for multi-column inserts (`NewUser<'a>`); never inline sqlx in a handler,
  page or procedure. Recipe: the `new-query` skill.
- **Configuration is parsed once** (`src/config.rs`) and read with
  `config(cx)` / `app_url(cx, path)`; nothing else reads env vars (except
  `DATABASE_URL` in `main.rs`).
- **Styling**: BEM classes are the contract (CSS targets only them, never
  element structure or ids); colors are role-named `light-dark()` variables
  in `variables.css`, never hard-coded; no inline styles; one breakpoint
  (`max-width: 760px`), check pages at 390px. Details in
  `.claude/rules/styling.md`.

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

## Layout

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
- `src/components/` — reusable UI, one file per component; each has a matching
  CSS file in `assets/css/components/` and a demo in `src/pages/styleguide/`.
  Import components by module path: `components::button::{button, ButtonSize}`
  (`#[component]` generates a module named after the function, so they can't
  be re-exported from `components.rs`). Available: `button` (Primary /
  Secondary / Danger), `form` controls (incl. `code_field`), `card` +
  `card_header` + `card_body` (every settings block), `badge`, `avatar`,
  `menu` (popover, opened by any button with `popovertarget`), `tabs` + `tab`
  (link tabs), `table` + `table_row`/`table_heading`/`table_cell`,
  `empty_state`, `toggle` (a `role="switch"` checkbox), `copy_field`,
  `modal`, `confirm_dialog` (destructive confirmations), `notice`, `toaster`,
  `icon`, `theme_switcher`. Use these before writing page-specific markup.
- `src/layout/` — header, nav, footer, logo, page heading.
- `src/pages/` — general pages (`#[page("/path")]`); `src/pages/styleguide/` —
  one section per component.
- `src/user/` — `pages/` (pages, POST routes, account components; email
  verification in `verify_email.rs`), `auth.rs` (`current_user`,
  `require_user`, `sign_in`, `hash_password`), `queries`, `service` (email
  content), `tokens.rs` (`TokenPurpose`, `generate_token`), `passkeys.rs`.
- `src/web.rs` — `GlobalState`, `globals(cx)`, `session(cx)`, `db(cx)`,
  `inject_globals` axum middleware.
- `src/rate_limit.rs`, `src/cleanup.rs` (hourly), `src/flash.rs`,
  `src/utils.rs`.

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
| Form failure | `*Problem` enums | rendered after `rewrite(…)` |
| Theme | `Theme` + cookie | `POST /theme` or `window.setTheme` (`assets/js/theme.js`) |
| Account tab | `AccountTab` (`ALL`, `href()`) | URL → `from_slug`, unknown → 404 |
| Config | `Config`, `Mailer`, `RateLimitMode` | parsed once at startup |

When adding state, extend one of these or add a row here.

## Topcoat conventions (0.12)

- Every `#[component]`, `#[page]`, `#[route]`, `#[layout]` and `#[procedure]`
  takes `cx: &Cx` as its first parameter, named exactly `cx`, even if unused:
  `view!` captures it. Helper functions that need context take `cx: &Cx` too.
- One context: shared values are added with `.with(value)` on the router
  builder (`app.rs`) and read with `context::<T>(cx)` / `try_context::<T>(cx)`
  behind small helpers (`db(cx)`, `config(cx)`, `rate_limits(cx)` in
  `web.rs`). `rewrite(…).with(value)` carries a value to the page it rewrites
  to; the page reads it with `try_context`. `cx.with(value)` shadows a value
  for code given the child context.
- Axum extensions (`Session`, `GlobalState`) are still read with
  `request::extensions(cx)`.
- `path_param!(token)` generates `token(cx) -> &str`; there are no wrapper
  types. `href!(handler)` links to a handler; path values go in as
  `href!(handler, name: value)`.
- In component calls a comma separates the last prop from the first child:
  `button(busy: $(…), "Save")`.
- Page titles come from the `metadata` module: the root layout sets the site
  name, each page adds `with(Metadata::new().title(page_title("Login")))`
  (`app::page_title` appends " · Webserver"), and `topcoat::metadata::tags()`
  renders `<title>`, charset and viewport in `<head>`. Use
  `Metadata::from_fn` when the title depends on the request (see the account
  tab page).
- Router setup: `.assets(bundle).layer(RuntimeLayer::new())`; origin policy,
  body limits and compression are per-route `with(...)` values if ever needed.
- Pages, routes and the layout use explicit paths and are collected by
  `.discover()`; nothing needs registering by hand.
- `topcoat fmt` formats every Topcoat macro (`#[page]`, `href!`, `view!`…);
  run it rather than hand-wrapping macro arguments.

## Topcoat gotchas

- A procedure call rejects (it doesn't return `Err`) when the request fails
  or the server errors. Turn that into an `Err` with one `raw!`, using a
  fallback built in server-side Rust:
  `let save_failed: std::result::Result<String, String> = Err(…);` before
  `view!`, then inside the handler
  `let result = raw!("await Promise.resolve(${_pending}).catch(() => ${save_failed})", save_failed.clone());`
  (see `account_profile.rs`).
- Don't bind an input's `:value` to the same signal its `@input` writes: the
  binding rewrites the field whenever the signal changes. Keep a separate
  signal for `:value` that only changes when you deliberately reset the field
  (`field_value` vs `draft_name` in `account_profile.rs`).
- Closures in `attributes! { @click=$(...) }` need a typed param:
  `|_e: Event|`. Inside `view!` directly, `|_e|` is fine.
- `$(...)` supports only a subset of Rust (no string concatenation, no
  `format!`): precompute class strings on the server and capture them.
- `raw!("js", fallback)` quirks: always pass a fallback (`()`), or code after it
  won't compile; put it last in its block (Topcoat omits the trailing `;`, so a
  following statement starting with `(` breaks the JS); wrap `${binding}`
  values in `String(...)` before comparing — bindings are runtime wrapper
  objects, and handler events are wrapped too (use `e.current_target.value`,
  not `${e}.target`).
- An `await` inside `raw!` only compiles at the top level of an `async`
  handler, not inside an `if`/`match` block (the browser throws "Failed to
  compile event @click"). Await first, then branch on the result (see
  `copy_field.rs`).
- Don't use cross-document view transitions (`@view-transition`): Chromium
  flashes white between pages.
- After upgrading `topcoat-cli`, restart `topcoat dev`: an older CLI still
  running rebuilds with an incompatible bundler and writes an empty asset
  bundle, so every page panics with "failed to resolve asset".
- A missing asset bundle panics at startup; `topcoat dev` (or
  `topcoat asset bundle`) creates it in `target/<profile>/assets`.

## Development

From `webserver/`:

```sh
topcoat dev
```

Builds the app, bundles Topcoat's browser runtime, starts the server
(respects `HOST`/`PORT`, default `127.0.0.1:3000`) and live-updates the page on
changes (including `assets/css`). CLI:
`cargo install topcoat-cli --version 0.12.1 --locked`.

**Before calling work done, run `./check.sh`**: `cargo fmt --check`,
`topcoat fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, and the Playwright suite. Format with
`topcoat fmt --rustfmt $(find src -name "*.rs")` (formats Rust and `view!`
bodies; never use `leptosfmt`). While iterating, use the `verify` skill
(one spec, Chromium only, plus screenshots).

**Migrations**: add a *new* file in `migrations/` (never edit an applied one;
a hook blocks it), then `sqlx migrate run` against the dev database (sqlx
macros check queries against it at compile time). The test database is
migrated automatically by `webserver seed`. Query metadata for offline
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
  before a turn ends, `cargo clippy --all-targets -D warnings` runs (in
  `target/claude-check`, skipped when nothing changed); edits to applied
  migrations and generated files (`assets/dist/`, `target/`, `e2e/.mail`,
  `e2e/test-results/`) are blocked (reading them is fine, e.g. failure
  screenshots), and so is reading `.env`; session start reports branch, database,
  pending migrations and whether `topcoat dev` is running.
- Skills: `new-component`, `new-query`, `new-flash`, `new-form-flow`,
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
