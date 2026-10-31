# Webserver Project

Rust web app: axum + Leptos 0.8 in **islands mode**. Pages are server-rendered
Leptos components; only `#[island]` components ship WASM and hydrate in the
browser. Postgres via sqlx (with migrations), tower-sessions, argon2 auth. CSS
is bundled/minified by Lightning CSS in `build.rs` (`assets/css/` →
`assets/dist/app.css`, served from `/assets`).

## Architecture principles

- **Compile-time safety first**: components, props, routes, server functions and
  sqlx queries are all checked by the compiler. Component variants are Rust
  enums (`ButtonSize::Sm`), never strings.
- **Native platform over JS**: use native HTML/CSS features where they exist —
  `<dialog>` + `command`/`commandfor` for modals, `popover`, CSS transitions.
- **State lives at the lowest rung that works**: server data → server-rendered
  components and `#[server]` functions; interactive widget state → signals
  inside the smallest possible `#[island]`; cross-island communication → window
  events (e.g. `dispatch_toast("toast")`), not shared client state.
- **BEM classes are the styling contract**: components emit stable class names
  (e.g. `a-notice--success`); CSS targets only those, never element/tag structure
  (the one exception: `leptos-island, leptos-children { display: contents }`).

## Layout

- `src/main.rs` — server binary (`ssr` only): sessions, db pool, router.
- `src/lib.rs` — crate root + `hydrate()` WASM entry point.
- `src/app.rs` — `shell` (HTML document) and `App` (`<Routes>`; the list of pages).
- `src/components/` — reusable UI, one file per component; each has a matching
  CSS file in `assets/css/components/` and a demo in `src/pages/styleguide/`.
- `src/layout/` — header, nav, footer, logo, page heading.
- `src/pages/` — general pages; `src/pages/styleguide/` — one section per component.
- `src/user/` — `api.rs` (`#[server]` functions), `pages/` (components),
  plus server-only `auth`, `queries`, `service`, `controller` (email verify link).
- `src/web.rs` — `AppState`, `GlobalState` + `use_globals()`, `inject_globals` middleware.

## Leptos gotchas

- The crate builds twice: `ssr` (server binary) and `hydrate` (WASM lib).
  Server-only deps are `optional` and enabled by `ssr`; gate server-only code
  with `#[cfg(feature = "ssr")]` or put it inside a `#[server]` fn body.
- Event handlers (`on:click`) only work inside an `#[island]`; in a plain
  `#[component]` they're silently dropped (it never runs in the browser).
- Island props must be `Serialize + Deserialize`, and can't use `default = expr`
  or `optional` on non-`Option` types. Pattern: a public `#[component]` with
  friendly defaults that renders a private `#[island]` (see `Toaster`).
- Island `children` are opaque server HTML: render them unconditionally and hide
  with CSS rather than `<Show>`.
- `leptos_axum::redirect()` only works before headers are sent: use
  `ssr=SsrMode::Async` on the route or `<Await blocking=true>`.
- Server function return values are sent to the browser — return narrow DTOs
  (`AccountUser`), never `User` (it holds the password hash).
- Deep `view!` trees can hit the type recursion limit; `lib.rs` sets
  `recursion_limit = "256"`; `.into_any()` also erases types at a boundary.

## Development

Run the dev watch loop from `webserver/`:

```sh
cargo leptos watch
```

This rebuilds the server binary and the WASM bundle when `src/`, `assets/css`
or `assets/icons` change. Tests: `cargo test --features ssr`.
`cargo-leptos` is pinned at 0.3.10 (0.3.11 needs rustc 1.96).
