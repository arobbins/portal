---
paths:
  - "src/**/*.rs"
---

# Topcoat conventions (0.12)

- Every `#[component]`, `#[page]`, `#[route]`, `#[layout]` and `#[procedure]`
  takes `cx: &Cx` as its first parameter, named exactly `cx`, even if unused:
  `view!` captures it. Helper functions that need context take `cx: &Cx` too.
- One context value: `App` is added with `.with(app)` on the router
  builder (`app.rs`) and read only through `web::app(cx)`, `web::env(cx)`
  and `web::config(cx)` (`web/context.rs`; `context::<…>` anywhere else
  fails the clean-code check). `web::redisplay(path, failure)` re-renders
  the GET page with `failure` in context; the page reads it with
  `try_context::<Failure>(cx)`. `cx.with(value)` shadows a value
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
- After upgrading `topcoat-cli` or the Rust toolchain, restart `topcoat dev`: an older CLI still
  running rebuilds with an incompatible bundler and writes an empty asset
  bundle, so every page panics with "failed to resolve asset".
- A missing asset bundle panics at startup; `topcoat dev` (or
  `topcoat asset bundle`) creates it in `target/<profile>/assets`.

View, signal and `raw!` gotchas: `topcoat-gotchas.md`.
