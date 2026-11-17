---
paths:
  - "src/**/pages/**"
  - "src/pages/**"
  - "src/components/**"
  - "src/layout/**"
  - "src/flash.rs"
  - "src/app.rs"
---

# Page state, toasts and flashes

- **Native platform over JS**: use native HTML/CSS features where they
  exist — `<dialog>` + `command`/`commandfor` for modals, `popover`, CSS
  transitions and animations (the toaster auto-hides via a CSS animation +
  `@animationend`).
- **State lives at the lowest rung that works**: server data → render it in
  the page; widget state → a `signal(cx, ...)` inside the component; shared
  widget state → create the signal in the common parent and pass
  `&Signal<T>` down (see the styleguide toaster section). Prefer one signal
  holding *which* state you're in over several booleans that must agree.
- **Toasts**: one `toast: Signal<String>` per page holding the visible
  toast's name (or `NO_TOAST`); `toaster(toast: &toast, name: …, …)` shows
  itself when it matches, so only one toast can ever be open. Build the
  signal with `flash_toast(cx, flash)`, or `page_toast(cx, failed.is_some(),
  flash)` on a page with a modal form. Other toast names are per-page
  `const`s, never literals. Each toaster is a `role="status"` live region and a closed
  toast's message is `visibility: hidden`, so screen readers announce a toast
  when it opens and never read closed ones. Errors inside a `<dialog>` go in the dialog, not
  a toast (an open modal covers toasts).
- **Success messages after a redirect are session flashes** (`src/flash.rs`):
  `set_flash(&session, Flash::ProfileSaved)` before `see_other(...)`; the
  target page calls `take_flash(cx, FLASHES)` (a page `const` listing what
  it accepts), which removes it only if it's one the page accepts, and
  renders `flash_toasts(toast: &toast, flashes: FLASHES)`. JS shows the same
  toast with `Flash::X.name()`. Never put message flags in the URL (they
  reappear on refresh and can be faked). New messages: the `new-flash` skill.
- **Forms are plain HTML POSTs**: a `#[route(POST ...)]` handler either
  `see_other(...)` redirects on success or returns `redisplay(path,
  failure)` (`web/context.rs`), which re-renders the GET page with a
  request-context value carrying the error (see `auth/pages/login.rs`).
  They work without JS; `@submit` only adds the busy state. Details in
  `forms.md`.
- **Failure reasons are enums** implementing `ActionProblem`; render
  `(page_status(problem))` and `(problem.message())`, never compare message
  text.
