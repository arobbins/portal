# Webserver Project

Rust web app: axum + [Topcoat](https://github.com/tokio-rs/topcoat) 0.11. Axum
owns the server (sessions, middleware, `/assets`), and hands every other
request to a Topcoat router through `TowerService` as its fallback. Topcoat renders all pages on the server; interactivity comes from
small `$(...)` runtime expressions that Topcoat compiles to JavaScript (no
WASM). Postgres via sqlx (with migrations), tower-sessions, argon2 auth. CSS is
bundled/minified by Lightning CSS in `build.rs` (`assets/css/` →
`assets/dist/app.css`, served from `/assets`).

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
  (see the styleguide toaster section).
- **Forms are plain HTML POSTs**: a `#[route(POST ...)]` handler either
  `see_other(...)` redirects on success or `rewrite(...)`s back to the GET page
  with a request-context value carrying the error (see `user/pages/login.rs`).
  They work without JS; `@submit` only adds the busy state.
- **BEM classes are the styling contract**: components emit stable class names
  (e.g. `a-notice--success`); CSS targets only those, never element structure.
- **Colors are role-named variables** in `assets/css/variables.css`
  (`--color-text`, `--color-surface-card`, `--border-control`, …), each
  defined once as `light-dark(<light>, <dark>)`. Never hard-code a color in a
  component file; never name a variable by lightness.
- **Theming**: `color-scheme` on `<html>` picks the `light-dark()` side.
  `data-theme` is `light`, `dark` (default) or `system` (follows the OS). The
  server reads the `theme` cookie in `inject_globals` (`src/theme.rs`) and
  renders `data-theme` + `<meta name="color-scheme">`, so there's no flash of
  the wrong theme. `theme_switcher` switches live via JS and falls back to
  `POST /theme` without JS.

## Layout

- `src/main.rs` — axum server: sessions, db pool, `/assets`, the Topcoat
  fallback, and the `webserver seed` subcommand.
- `src/mail.rs` — `Mailer` (`postmark` | `log` | `file`, from `MAIL_TRANSPORT`),
  in Topcoat app context as `mailer(cx)`.
- `src/seed.rs` — `webserver seed`: creates/migrates/resets a `*_test` database
  and inserts the e2e users (refuses any other database name).
- `src/theme.rs` — `Theme` enum and `theme` cookie parsing.
- `src/app.rs` — Topcoat router builder, root `#[layout("/")]` (HTML document,
  header, footer, 404 error boundary), `not_found!("/")`.
- `src/components/` — reusable UI, one file per component; each has a matching
  CSS file in `assets/css/components/` and a demo in `src/pages/styleguide/`.
  Import components by module path: `components::button::{button, ButtonSize}`
  (`#[component]` generates a module named after the function, so they can't
  be re-exported from `components.rs`).
- `src/layout/` — header, nav, footer, logo, page heading.
- `src/pages/` — general pages (`#[page("/path")]`); `src/pages/styleguide/` —
  one section per component.
- `src/user/` — `pages/` (pages, POST routes, account components; email
  verification + resend in `verify_email.rs`), `auth.rs` (`current_user`,
  `require_user`, `sign_in`, `hash_password`), `queries`, `service` (email
  content), `tokens.rs` (`TokenPurpose`, `generate_token`).
- **Registration is email-first**: `/register` takes name + email (+ a hidden
  `website` honeypot) and always redirects to `/register?sent=true`, whatever
  the email's state (new → user with `password = NULL` + link; unverified →
  fresh link; verified → "you already have an account" email), so it never
  reveals whether an account exists. The link (`GET /user/verify/{token}`)
  only *shows* the set-password form; `POST` to the same URL consumes the
  token, sets the password, verifies and signs in. GETs never consume tokens
  (email scanners prefetch links). Users without a password can't log in.
- **Rate limits** (`src/rate_limit.rs`, `governor`, in memory, read with
  `rate_limits(cx)`): logins 10/min per IP, sign-ups + resends 5 per 10 min per
  IP, and at most 3 emails per hour to any one address. Failed logins are
  counted per account (`FailureWindow`, only wrong email/password counts, a
  successful login clears it): 5 per 15 min per account+IP, 50 per hour per
  account from any IP — then even the right password gets "Too many attempts". Per-IP blocks show a
  friendly message with status 429; the per-email limit silently skips the
  email (same response, so it reveals nothing). `RATE_LIMITS=relaxed` (used by
  the e2e server) multiplies limits by 1000. Client IPs come from Topcoat's
  `client_ip(cx)` (the connection address via `attach_remote_addr`); **behind a
  proxy/load balancer set `TRUSTED_PROXY_HOPS`** to the number of proxies, or
  every visitor shares the proxy's IP. Counters reset on restart and aren't
  shared across servers.
- Form fields take typed `autocomplete: Some(Autocomplete::…)` hints (name,
  email, current/new password) for password managers.
- Emailed links use the `user_tokens` table: one row per token with a
  `purpose`, only the SHA-256 hash stored, single use (consumed with
  `DELETE … RETURNING`), expiry per purpose. Add a `TokenPurpose` variant for
  new flows (password reset, email change).
- `src/web.rs` — `GlobalState`, `globals(cx)`, `session(cx)`, `db(cx)`,
  `inject_globals` axum middleware.

## Topcoat gotchas

- Pages, routes and the layout use explicit paths and are collected by
  `.discover()`; nothing needs registering by hand.
- Axum request extensions are visible to Topcoat via `request::extensions(cx)` —
  that's how `Session` and `GlobalState` reach pages. The pool is Topcoat app
  context (`app_context::<PgPool>`, wrapped by `db(cx)`).
- Closures in `attributes! { @click=$(...) }` need a typed param:
  `|_e: Event|`. Inside `view!` directly, `|_e|` is fine.
- `$(...)` supports only a subset of Rust (no string concatenation, no
  `format!`): precompute class strings on the server and capture them.
- Every value coming back from the browser (signals, procedure/shard args) is
  user input — validate it. Shards/procedures bypass layout/page guards.
- `raw!("js", fallback)` quirks: always pass a fallback (`()`), or code after it
  won't compile; put it last in its block (Topcoat omits the trailing `;`, so a
  following statement starting with `(` breaks the JS); wrap `${binding}`
  values in `String(...)` before comparing — bindings are runtime wrapper
  objects, and handler events are wrapped too (use `e.current_target.value`,
  not `${e}.target`).
- Don't use cross-document view transitions (`@view-transition`): Chromium
  flashes white between pages.
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
`cargo install topcoat-cli --version 0.11.0 --locked`.

**Before calling work done, run `./check.sh`**: `cargo fmt --check`,
`topcoat fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, and the Playwright suite. Format with
`topcoat fmt --rustfmt $(find src -name "*.rs")` (formats Rust and `view!`
bodies; never use `leptosfmt`).

**Migrations**: add a file in `migrations/`, then `sqlx migrate run` against
the dev database (sqlx macros check queries against it at compile time). The
test database is migrated automatically by `webserver seed`.

**Email**: `MAIL_TRANSPORT` = `postmark` (default; needs `POSTMARK_*`), `log`
(prints emails, including links), or `file` (writes JSON to `MAIL_DIR`).

**Behavior tests** (Playwright, `e2e/`, Chromium + Firefox + WebKit):
`cd e2e && npm test` (headless), `npm run ui` (dashboard), `npm run report`.
Playwright runs its own server on **:3100**: it builds, runs
`webserver seed` against `<dev DATABASE_URL>_test` (or `TEST_DATABASE_URL`),
and uses the file mailer (`e2e/.mail`, cleared each run), so it never touches
the dev server or dev data. Conventions:
- Import `test`/`expect` from `e2e/support/test.ts`: any console error or
  uncaught exception fails the test; allow expected ones with
  `test.use({ allowedConsoleErrors: [/…/] })`.
- Shared helpers: `support/flows.ts` (`register`, `logIn`, `uniqueEmail`),
  `support/mail.ts` (`waitForEmails`, `verificationLink`), `support/users.ts`
  (seeded users — keep in sync with `src/seed.rs`). Tests that change data use
  their own `uniqueEmail` user, never the seeded ones.
- Find elements by role/label/text (`getByRole("button", { name: "Show toast" })`),
  not CSS classes; assert visible behavior. One spec file per feature.
