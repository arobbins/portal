# Webserver Project

Rust web server app: axum + Askama server-side rendering, HTMX for client/server
communication, Alpine for minor interactivity. Postgres via sqlx (with
migrations), tower-sessions, argon2 auth. CSS is bundled/minified by Lightning
CSS in `build.rs` (`assets/css/` → `assets/dist/app.css`).

## Architecture principles

- **Compile-time safety first**: Askama templates and sqlx queries are checked
  by `cargo build`. Prefer patterns that keep errors at compile time.
- **Native platform over JS**: use native HTML/CSS features where they exist —
  `<dialog>` + `command`/`commandfor` for modals, `popover`, CSS transitions.
- **State lives at the lowest rung that works**: server state → HTMX round-trips;
  widget-local state → `x-data` inside the component macro; no shared client
  state (no Alpine stores — deliberately rejected); hand-written web components
  only for genuinely rich widgets (currently none).
- **BEM classes are the styling contract**: components emit stable class names
  (e.g. `a-notice--success`); CSS targets only those, never element/tag structure.

## Migration status (web components → Askama macros)

- Ported to `ui.html` macros: `notice`, `toaster`, `button` (styleguide's
  Buttons section still uses `<a-button>` tags, though).
- Still web components: button, form, modal (`assets/js/components/`, registered
  in `app.js`); `utils.js` and the `:not(:defined)` FOUC CSS hacks go when they do.
- Not yet wired: server-driven toasts — toaster listens for a `toast` window
  event; handlers should fire it via `HX-Trigger` response headers. The old
  Alpine partial `templates/components/toaster.html` (account page) should fold
  into the macro.

## Development

Run the dev watch loop from `webserver/`:

```sh
cargo watch -w src -w templates -w assets -x run
```

This rebuilds and restarts the server whenever `.rs`, `.html`, `.js`, or `.css` files change — `cargo watch` can't filter by extension, so it watches the directories where those files live (`src`, `templates`, `assets`).

## UI components (Askama macros)

Reusable components live in `templates/ui.html` as `{% macro %}`s
(import per page: `{% import "ui.html" as ui %}`). Conventions and gotchas:

- Every `{% call %}` requires `{% endcall %}`, even with an empty body.
- `{% if %}` has no string truthiness — use `!arg.is_empty()` (bools/ints coerce, strings don't).
- A macro **cannot forward its `caller()` body into a nested `{% call %}` block**
  ("block is not defined for `caller`"), and binding it via `{% let %}` corrupts
  the output buffer. Composite macros (e.g. `toaster`) inline the child's BEM
  markup instead of calling the child macro.
- Alpine-reactive props pass expressions into directives (`xText` → `x-text`,
  `xData` → `x-data`); a standalone Alpine trigger needs `xData = "{}"` to activate.
