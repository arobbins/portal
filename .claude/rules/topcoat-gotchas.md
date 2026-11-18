---
paths:
  - "src/components/**"
  - "src/layout/**"
  - "src/*/pages/**"
  - "src/pages/**"
  - "src/app.rs"
---

# Topcoat view gotchas

- A procedure call rejects (it doesn't return `Err`) when the request fails
  or the server errors. Turn that into an `Err` with one `raw!`, using a
  fallback built in server-side Rust:
  `let save_failed: std::result::Result<String, FieldError> = Err(FieldError::general(UNREACHABLE));` before
  `view!`, then inside the handler
  `let result = raw!("await Promise.resolve(${_pending}).catch(() => ${save_failed})", save_failed.clone());`
  (modal forms get this from `modal_form` in `components/form/submit.rs`;
  `api_tokens/pages/create_token_dialog.rs` shows it standalone).
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
- Generic components (`modal_form<P: TypedProcedure>` in
  `components/form/submit.rs` is the reference): return a concrete type
  (`Result<impl View>` with a fixed `String` procedure output), not a generic
  `T`, or the trait solver overflows; bound `P::Args: Clone + Send + Sync +
  'static` and `<P as Surrogated>::Surrogate: Serialize`; pass procedure
  arguments as one tuple `Expr<P::Args>` prop built with `$(...)` at the call
  site (`$(...)` only works inside `view!`) and spread it in the handler.
  Avoid higher-ranked (`for<'a>`) bounds.
