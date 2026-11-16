---
paths:
  - "src/components/form.rs"
  - "src/*/pages/**"
  - "src/pages/**"
  - "assets/css/components/form.css"
---

# Forms

- **Submit busy state**: pass `submitting: Some(&is_submitting)` to `form(…)`
  instead of writing your own `@submit`; it sets the busy state and reloads
  the page if the browser restores it from the back/forward cache (otherwise
  the button would stay disabled after pressing Back). Give the button
  `busy: $(is_submitting.get())` only: a busy button is always disabled.
- **Form controls** (`components/form/`, re-exported from `components::form`): `form_field`
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
- **Forms submit to actions**: the route's `Form<…>` is the action's input
  struct, and failures are the action's `ActionProblem`; pages never
  validate or query. Field `name`s are the constants the problem's `field()`
  returns.
- **Form errors**: render the problem with `form_error((problem.message()))`
  just above the submit button, and pass `invalid: true` to the fields it's
  about (from `problem.field()` or an `invalid_*()` helper). The page status
  is `page_status(problem)`. `form_error` has
  `role="alert"` and the id `FORM_ERROR_ID`, which invalid fields reference
  through `aria-describedby`; for JS-driven errors pass
  `hidden: $(message.is_empty())`.
- Values the user can see but not change (the current email in Change
  email) are `readonly`, not `disabled`: readable, selectable and announced;
  `.a-form-input[readonly]` styles them. In a dialog, put `autofocus` on the
  first editable field so focus skips the read-only one.
- Form fields take typed `autocomplete: Some(Autocomplete::…)` hints (name,
  email, current/new password) for password managers.
- **Modal forms** share one scaffold, one `DialogForm` per dialog (never
  shared between dialogs): `DialogForm::new(cx, failure)` takes that
  dialog's own no-JS failure (`Option<ProblemSpec>`) and holds the error
  field, error message and busy signals; inputs spread `dialog_input(cx,
  &form, FIELD, &value, &draft)` and the error is `dialog_form_error(form:
  &form)`. The page shows whichever dialog failed in one error toast,
  `dialog_error_toast(failure: page_failure, toast: &toast)`, with the toast
  signal from `page_toast(cx, page_failure.is_some(), flash)`. The dialog itself is
  `modal_form(…, submit: ModalSubmit { … }, args: $((…)))`
  (`form/submit.rs`), which owns the procedure call, busy state, catch,
  success toast and closing; dialogs that don't close on success (Create
  API token) still write their own `@submit`.
- The full recipe for a new form (page, POST route, problem enum, modal +
  procedure) is the `new-form-flow` skill.
