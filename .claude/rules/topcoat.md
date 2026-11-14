---
paths:
  - "src/**/*.rs"
---

# Topcoat conventions (0.12)

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

# Topcoat gotchas

- A procedure call rejects (it doesn't return `Err`) when the request fails
  or the server errors. Turn that into an `Err` with one `raw!`, using a
  fallback built in server-side Rust:
  `let save_failed: std::result::Result<String, FieldError> = Err(FieldError::general(UNREACHABLE));` before
  `view!`, then inside the handler
  `let result = raw!("await Promise.resolve(${_pending}).catch(() => ${save_failed})", save_failed.clone());`
  (see `profile/pages/profile_tab.rs`).
- Don't bind an input's `:value` to the same signal its `@input` writes: the
  binding rewrites the field whenever the signal changes. Keep a separate
  signal for `:value` that only changes when you deliberately reset the field
  (`field_value` vs `draft_name` in `profile/pages/profile_tab.rs`).
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
- After upgrading `topcoat-cli` or the Rust toolchain, restart `topcoat dev`: an older CLI still
  running rebuilds with an incompatible bundler and writes an empty asset
  bundle, so every page panics with "failed to resolve asset".
- A missing asset bundle panics at startup; `topcoat dev` (or
  `topcoat asset bundle`) creates it in `target/<profile>/assets`.
