---
name: new-form-flow
description: Build a complete form feature in this app — GET page, POST route that redirects or rewrites with a typed problem, optional modal + procedure for the JS path, flash on success, and its e2e spec. Use whenever adding a form (settings card, sign-up style page, any submit).
---

# New form flow

Every form submits to an **action** (the `new-action` skill): the page never validates or
queries itself. References: stand-alone page form → `src/auth/pages/login.rs`; settings
card with modal + procedure → `src/profile/pages/profile_tab.rs`. Read the one that
matches before starting.

## 1. Action first

Write the action, its input struct (`Deserialize` + `JsonSchema`, typed fields) and its
problem enum (`ActionProblem`, with `field()` naming the form field) with the `new-action`
skill. The input struct is the route's `Form<…>`; only browser-only extras (a honeypot)
get a separate form struct that builds the input (see `auth/pages/register.rs`).

Field names are constants shared by the problem's `field()` and the form
(`PASSWORD_FIELD`, `CURRENT_PASSWORD_FIELD`, `NAME_FIELD`).

## 2. Failure context

```rust
struct ThingFailed { field: String, problem: ThingProblem }
```

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

- Render `(page_status(problem))` when there's a problem: 200 for problems the visitor
  can fix (the form is redisplayed), 429 for rate limits. JSON responses keep the
  problem's real status.
- `form(submitting: Some(&is_submitting), variant: FormVariant::Page /* or default Inline */,
  attrs: attributes! { method="post" action=(href!(thing_submit)) }, ...)`.
- Fields from `components/form.rs` with `id`, `name: FIELD_CONST`, typed `autocomplete`,
  `value` from `failed` so input survives an error, and `invalid:` from the problem.
- `form_error((problem.message()))` right above `form_actions(button(button_type:
  ButtonType::Submit, busy: $(is_submitting.get()), "Save thing"))`.
- Signed-in pages call `require_user(cx)?`; pages for guests call
  `redirect_signed_in_users(cx)?`.

## 4. POST route (works without JS)

```rust
#[route(POST "/account/things/update")]
pub async fn update_thing_submit(cx: &Cx, Form(input): Form<UpdateThing>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    match update_thing(&env(cx), &caller, &input).await? {
        Ok(_) => {
            set_flash(&session(cx)?, Flash::ThingSaved).await?;
            Ok(see_other(AccountSection::Things.path()))
        }
        Err(error) => {
            let failed = ThingFailed { field: input.field, problem: error.for_browser()? };
            Err(redisplay(AccountTab::Things.path(), failed))
        }
    }
}
```

Account routes are `/account/<thing>/<verb>`; signed-out forms post to their page's URL.
Never type a path: `href!(handler)`, `AccountTab::X.path()`, `AccountSection::X.path()`,
`routes.rs` constants. Add the flash with the
`new-flash` skill.

## 5. Modal + procedure (settings cards only)

Follow `profile/pages/profile_tab.rs` exactly (and `account/pages/security_tab.rs` +
`password_routes.rs` for the file split once a tab grows past 300 lines):

- `let form = DialogForm::new(cx, "", "")` holds the error field, error message and busy
  signals; inputs take `attrs: dialog_input(cx, &form, FIELD, &value, &draft)`, the
  dialog's error is `dialog_form_error(form: &form)`, and the fallback's toast is
  `dialog_error_toast(form: &form, toast: &toast, name: ERROR_TOAST)`.

- `card(id: Some(AccountSection::X.anchor()), …)` with the action button in its header
  (`command="show-modal" commandfor=(MODAL_ID)`; its `@click` resets the field signals).
  The button label and the modal heading are the same "Verb noun" ("Edit profile",
  "Change password", "Add passkey", "Create API token").
- Two signals for the input: `field_value` (bound to `:value`) and `draft_name` (written by
  `@input`). Never bind `:value` to the signal `@input` writes.
- The procedure returns `Result<Result<T, FieldError>>` (see `new-action`): `FieldError` is
  a `#[record]` with `field` and `message`, so the handler reads `failure.field` /
  `failure.message`.
- `@submit=$(async |e: Event| { e.prevent_default(); ... })` sets busy, calls the procedure,
  turns a rejected call into `Err(FieldError::general(UNREACHABLE))` with the
  `raw!(... .catch(() => ${save_failed}))` pattern, then on success sets the toast to the
  flash name and closes the dialog with `raw!`; on failure writes the message into the
  dialog's `form_error` (`hidden: $(error.get().is_empty())`) and sets `:aria-invalid` on
  the field whose name matches `failure.field`.
- Keep the POST route as the no-JS fallback; its error comes back as an error toast.

## 6. Tests and finish

- Write the spec with the `new-e2e-test` skill: success path, each problem message, the
  no-JS/rewrite path keeps the typed value, and for modals the "server can't be reached" case
  (`page.route("**/_topcoat/runtime/procedures/**", r => r.abort())`).
- Add a row to the CLAUDE.md state table if the flow adds state with more than two values.
- Run the `verify` skill, then `./check.sh`, then the `convention-reviewer` agent.
