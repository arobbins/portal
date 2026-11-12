---
name: new-form-flow
description: Build a complete form feature in this app — GET page, POST route that redirects or rewrites with a typed problem, optional modal + procedure for the JS path, flash on success, and its e2e spec. Use whenever adding a form (settings card, sign-up style page, any submit).
---

# New form flow

References: stand-alone page form → `src/user/pages/login.rs`; settings card with modal +
procedure → `src/user/pages/account_profile.rs`. Read the one that matches before starting.

## 1. Input, problem enum, failure context

```rust
#[derive(Deserialize)]
pub struct ThingInput { field: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThingProblem { Missing, TooLong, TooManyAttempts }

impl ThingProblem {
    fn invalid_field(self) -> bool { matches!(self, Self::Missing | Self::TooLong) }
    fn status(self) -> StatusCode { match self { Self::TooManyAttempts => StatusCode::TOO_MANY_REQUESTS, _ => StatusCode::OK } }
    fn message(self) -> &'static str { match self { /* user-facing sentences */ } }
}

struct ThingFailed { field: String, problem: ThingProblem }
```

- Validation that other forms share goes in its own module (`user/name.rs`,
  `user/password.rs` style) returning the problem enum.
- Never compare message text; branch on variants.

## 2. Validation + save in one function

One `async fn` used by both the POST route and the procedure, returning
`Result<Result<T, ThingProblem>>` (outer: server errors, inner: user problems). It validates
(including `parse_email` for emails, rate limits for password/email-sending actions), then
calls a `query_*` function (see the `new-query` skill). Never put sqlx here.

## 3. GET page

```rust
#[page("/path", with(Metadata::new().title(page_title("Title"))))]
pub async fn thing_page(cx: &Cx) -> Result<impl View> {
    let failed = try_context::<ThingFailed>(cx);
    let problem = failed.map(|f| f.problem);
    let is_submitting = signal(cx, || false);
    ...
}
```

- Render `(problem.status())` when there's a problem.
- `form(submitting: Some(&is_submitting), variant: FormVariant::Page /* or default Inline */,
  attrs: attributes! { method="post" action=(href!(thing_submit)) }, ...)`.
- Fields from `components/form.rs` with `id`, `name`, typed `autocomplete`, `value` from
  `failed` so input survives an error, and
  `invalid: problem.is_some_and(ThingProblem::invalid_field)`.
- `form_error((problem.message()))` right above `form_actions(button(button_type:
  ButtonType::Submit, busy: $(is_submitting.get()), "Save"))`.
- Signed-in pages call `require_user(cx)?`; pages for guests call
  `redirect_signed_in_users(cx)?`.

## 4. POST route (works without JS)

```rust
#[route(POST "/path")]
pub async fn thing_submit(cx: &Cx, Form(input): Form<ThingInput>) -> Result<SeeOther> {
    match save_thing(cx, &input.field).await? {
        Ok(_) => {
            set_flash(&session(cx)?, Flash::ThingSaved).await?;
            Ok(see_other(TARGET_URL))
        }
        Err(problem) => Err(rewrite(PAGE_URL, Body::empty())
            .method(Method::GET)
            .with(ThingFailed { field: input.field, problem })
            .into()),
    }
}
```

Add the flash with the `new-flash` skill.

## 5. Modal + procedure (settings cards only)

Follow `account_profile.rs` exactly:

- Card `account__content-card` with the action button in its header
  (`command="show-modal" commandfor=(MODAL_ID)`; its `@click` resets the field signals).
- Two signals for the input: `field_value` (bound to `:value`) and `draft_name` (written by
  `@input`). Never bind `:value` to the signal `@input` writes.
- `#[procedure] async fn save_thing_js(cx: &Cx, value: String) -> Result<Result<String, String>>`
  re-checks `current_user(cx)` (procedures bypass page guards) and calls the shared function.
- `@submit=$(async |e: Event| { e.prevent_default(); ... })` sets busy, calls the procedure,
  turns a rejected call into `Err` with the `raw!(... .catch(() => ${save_failed}))` pattern,
  then on success sets the toast to the flash name and closes the dialog with `raw!`; on
  failure writes the error into the dialog's `form_error` (`hidden: $(error.get().is_empty())`)
  and sets `:aria-invalid` on the field.
- Keep the POST route as the no-JS fallback; its error comes back as an error toast.

## 6. Tests and finish

- Write the spec with the `new-e2e-test` skill: success path, each problem message, the
  no-JS/rewrite path keeps the typed value, and for modals the "server can't be reached" case
  (`page.route("**/_topcoat/runtime/procedures/**", r => r.abort())`).
- Add a row to the CLAUDE.md state table if the flow adds state with more than two values.
- Run the `verify` skill, then `./check.sh`, then the `convention-reviewer` agent.
