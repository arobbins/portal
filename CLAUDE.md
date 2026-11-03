# Webserver Project

Rust web app: axum + [Topcoat](https://github.com/tokio-rs/topcoat) 0.11. Axum
owns the server (sessions, middleware, the email-verify route, `/assets`), and
hands every other request to a Topcoat router through `TowerService` as its
fallback. Topcoat renders all pages on the server; interactivity comes from
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

## Layout

- `src/main.rs` — axum server: sessions, db pool, `/user/verify/{token}`,
  `/assets`, and the Topcoat fallback.
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
- `src/user/` — `pages/` (pages, POST routes, account components), `auth.rs`
  (`current_user`, `require_user`, `sign_in`), `queries`, `service`,
  `controller` (axum email verify handler).
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
- A missing asset bundle panics at startup; `topcoat dev` (or
  `topcoat asset bundle`) creates it in `target/<profile>/assets`.

## Development

From `webserver/`:

```sh
topcoat dev
```

Builds the app, bundles Topcoat's browser runtime, starts the server
(respects `HOST`/`PORT`, default `127.0.0.1:3000`) and live-updates the page on
changes (including `assets/css`). Tests need the bundle once:
`topcoat asset bundle && cargo test`. Format view macros with
`topcoat fmt`. CLI: `cargo install topcoat-cli --version 0.11.0 --locked`.
