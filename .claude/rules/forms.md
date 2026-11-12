---
paths:
  - "src/components/form.rs"
  - "src/user/pages/**"
  - "src/pages/**"
  - "assets/css/components/form.css"
---

# Forms

- **Submit busy state**: pass `submitting: Some(&is_submitting)` to `form(…)`
  instead of writing your own `@submit`; it sets the busy state and reloads
  the page if the browser restores it from the back/forward cache (otherwise
  the button would stay disabled after pressing Back). Give the button
  `busy: $(is_submitting.get())` only: a busy button is always disabled.
- **Form controls** (`components/form.rs`): `form_field`
  (text/email/password), `textarea_field`, `select_field` (optional
  placeholder option), `radio_group` and `checkbox_group` (a `<fieldset>` +
  `<legend>`; options are `&[Choice { value, label }]`; option ids are
  `{id}-{value}`), and a single `checkbox`. Every control takes `id`, `name`,
  `invalid` and `attrs` (extra attributes for the control itself). Single
  fields are wrapped in a `<div class="a-form-group">`; only groups use
  `<fieldset>`.
- **Form layout**: `form(variant: FormVariant::Page, …)` for a stand-alone
  page form (narrow, centred), the default `Inline` everywhere else; never
  pass layout classes as strings. Classes all use the `a-form` prefix. Wrap
  the submit button (and any links beside it) in `form_actions(…)`. Form
  spacing comes from `--form-gap`, `--form-label-gap`, `--form-choice-gap`
  and `--form-column-gap` in `variables.css`; don't hard-code spacing in
  `form.css`.
- **Form errors**: render the problem with `form_error((problem.message()))`
  just above the submit button, and pass `invalid: true` to the fields it's
  about (each failure enum has `invalid_*()` helpers). `form_error` has
  `role="alert"` and the id `FORM_ERROR_ID`, which invalid fields reference
  through `aria-describedby`; for JS-driven errors pass
  `hidden: $(message.is_empty())`.
- Form fields take typed `autocomplete: Some(Autocomplete::…)` hints (name,
  email, current/new password) for password managers.
- The full recipe for a new form (page, POST route, problem enum, modal +
  procedure) is the `new-form-flow` skill.
