---
name: new-action
description: Add something the app can do as one shared action that the UI, the JSON API and the MCP endpoint all call — input/output types with JSON Schema, a problem enum with codes, the scope check, the event log, the page wiring, the /api/v1 route and tests. Use whenever adding or changing a feature that reads or changes user data.
---

# New action

Every feature ships to the UI and the API together. Read `.claude/rules/api.md` first.
Reference implementations: `src/profile/actions.rs` (scoped, has an API route),
`src/api_tokens/actions.rs` and `src/passkeys/actions.rs` (browser-only),
`src/auth/actions.rs` (public actions that run before sign-in).

## 1. Where it lives

A feature module `src/<feature>.rs` + `src/<feature>/`: `actions.rs`, `queries.rs`,
`pages/` (one file per page or account tab), `emails.rs` if it sends mail. `user/` holds
only the `User` model, its queries, `name.rs` and the event log.

## 2. Decide access

- Add (or reuse) a `Permission` in `src/permission.rs` and decide its `api_scope()`:
  reuse `<area>:read` / `<area>:write`, or add a scope pair (enum variant with
  `#[serde(rename = "area:read")]`, `ALL`, `label`, `choice_label`, `implies`, and a
  migration extending the `api_tokens.scopes` CHECK). No wildcard scopes.
- Security-sensitive (password, email, passkeys, tokens, deleting the account, billing
  ownership)? Its permission's `api_scope()` is `None`: tokens are denied and it gets
  **no** API route or MCP tool.
- Decide which `WorkspaceRole`s grant it (`WorkspaceRole::grants`).
- Runs before anyone is signed in (sign-in, sign-up, reset)? It's a public action: no
  `Caller`, returns `Outcome<T, Problem>`, does its own rate limiting.

## 3. Types in `<feature>/actions.rs`

```rust
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "What this does, for a model reading the schema.")]
pub struct DoThing {
    #[schemars(description = "…", length(min = 1, max = MAX_NAME_LENGTH))]
    pub name: String,
    #[schemars(description = "…")]
    pub kind: ThingKind,
    #[schemars(description = "The thing's id (thg_…).")]
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ThingView { /* only what a caller should see, never a DB model */ }
```

- Inputs are typed: choices are enums with `#[serde(rename = "…")]` values, never a
  `String` parsed later. The same struct is the `Form<…>` of the POST route, the JSON body
  and the MCP arguments.
- Limits come from constants (`MAX_NAME_LENGTH`, `MIN_PASSWORD_LENGTH`), never literals.
- Rows other callers can reference get a public id (`secret::PublicIdKind`, `tok_…`,
  `pk_…`): add a variant, a `public_id` column with a format CHECK, and use it in forms,
  URLs, events and the API. Integer ids stay internal.

Problems: an enum implementing `ActionProblem` with one `spec()` per variant:
`ProblemSpec::new("snake_case_code", "Message.").on_field(FIELD).with_status(…)` (code
never renamed once shipped; field names the input; status only if not 422). Wrapped
problems return `problem.spec()`. Wrap shared
problems instead of copying them: `NameProblem`, `PasswordProblem`,
`CurrentPasswordProblem` (via `check_current_password`), `TooManyAttempts`.

## 4. The action

```rust
pub async fn do_thing(env: &Env<'_>, caller: &Caller, input: &DoThing) -> ActionResult<ThingView, ThingProblem> {
    authorize!(caller.require(Permission::EditThings));
    let name = match check_name(&input.name) {
        Ok(name) => name,
        Err(problem) => return fail(ThingProblem::Name(problem)),
    };
    let mut tx = env.db().begin().await?;
    // query_* functions only (new-query skill) on &mut *tx, scoped by caller.owner_id()
    record_event(env, &EventSource::of(caller), AccountEventKind::ThingCreated,
        json!({ "thing": public_id, "name": name }), &mut *tx).await?;
    tx.commit().await?;
    Ok(Ok(view))
}
```

- Every write records an event inside the write's transaction; `detail` is a JSON object
  (ids and names, never secrets). A new `AccountEventKind` touches four places: the
  variant + `ALL` and its name (`src/user/events.rs`), a migration extending the
  `account_events_kind_check`, and its label in `fixed_summary`
  (`src/activity/actions.rs`). `check_conventions.py` catches a CHECK mismatch.
- Owned rows are scoped by `caller.owner_id()` (the workspace; today the user).
- Emails are jobs: `enqueue(&SendEmail(template.to(&email)), &mut *tx)` inside the
  transaction, with the email built by `mail::EmailTemplate`. Other network calls only
  after `commit`.
- Time is `env.now()`; the action never takes `cx` or the session. Keep it ≤ 40 lines:
  validation goes in a `check_request(...) -> Outcome<…>` helper, writes in a
  `store_*` helper (`api_tokens/actions/create.rs`). One use case per file under
  `<feature>/actions/`.

## 5. Page wiring

```rust
#[procedure]
async fn do_thing_js(cx: &Cx, name: String) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    match do_thing(&env(cx), &caller, &DoThing { name }).await? {
        Ok(view) => Ok(Ok(view.name)),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/things/create")]
pub async fn create_thing_submit(cx: &Cx, Form(input): Form<DoThing>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    match do_thing(&env(cx), &caller, &input).await? { … }
}
```

POST routes under `/account` are `/account/<thing>/<verb>`; forms for signed-out pages post
to the page's own URL. Redirect targets come from `href!(…)`, `AccountTab::X.path()`,
`AccountSection::X.path()` or a `routes.rs` constant. The procedure and POST route go in
`<feature>/pages/<thing>_routes.rs`. The rest follows `new-form-flow`.

## 6. Expose it to the API (actions a token may call)

Implement `ApiAction` in `<feature>/api.rs` (`profile/api.rs` is the reference): one
declaration drives the route handler, the `/docs/api` page, the OpenAPI document,
`/llms.txt` and the MCP tool list. A first use of an `HttpMethod` (e.g. `Delete`) also
needs its arms in `api/catalog.rs` (`as_str`, class name), its `Tone` and class in
`pages/api_docs.rs`, `--color-code-method-<verb>` in `variables.css` with its
styleguide token class, and `p-api-docs__code-method--<verb>` in `api-docs.css`.

```rust
pub struct CreateThingAction;

impl ApiAction for CreateThingAction {
    const ID: &'static str = "create-thing";
    const GROUP: &'static str = "Things";
    const METHOD: HttpMethod = HttpMethod::Post;
    const PATH: &'static str = "/api/v1/things";
    const SUMMARY: &'static str = "Create a thing";
    const DESCRIPTION: &'static str = "What it does, what it returns, what it records.";
    const PERMISSION: Permission = Permission::EditThings;

    type Input = DoThing;          // `NoInput` for GET
    type Output = ThingView;
    type Problem = ThingProblem;   // `Infallible` if it can't fail

    async fn run(env: &Env<'_>, caller: &Caller, input: &DoThing) -> ActionResult<ThingView, ThingProblem> {
        do_thing(env, caller, input).await
    }

    fn example_input() -> Option<DoThing> { Some(/* real-looking values */) }
    fn example_output() -> ThingView { /* … */ }
}

impl ListedProblem for ThingProblem {
    fn all() -> &'static [Self] { &Self::ALL }
}
```

Add it to `tools()` in `src/server/tools.rs`, then the thin route in
`src/api/v1/<feature>.rs` (Topcoat needs the path in the attribute):

```rust
#[route(POST "/api/v1/things", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn create_thing_route(cx: &Cx, body: Body) -> Result<ApiResponse> {
    serve_with_body::<CreateThingAction>(cx, body).await   // `serve::<A>(cx)` for GET
}
```

Add `pub mod <feature>;` to `src/api/v1.rs`. The scope, body and problem errors are
documented automatically; `scripts/check_conventions.py` fails if a `/api/v1` route has no
registered `ApiAction`, and `server/tests.rs` tests that every registered endpoint exists and needs
a token.

Lists: cursor pagination. Creates: accept `Idempotency-Key` (build the shared mechanism
the first time it's needed).

## 7. Tests

- Rust (`#[sqlx::test]`, `let app = App::for_tests(db.clone()); let env = app.env(None);`
  calls the action directly; set `app.clock = Clock::Fixed(at)` for time-dependent logic): the scope matrix (browser ok, token without the scope →
  `Denied::MissingScope`, token with it ok, browser-only actions → `Denied::BrowserOnly`
  with nothing written and no event), a schema test (`schemars::schema_for!`: required
  fields, limits, `additionalProperties: false`), and a `*_matches_the_limit` test for any
  number written into a message.
- e2e (`api-<feature>.spec.ts`, helpers in `support/api.ts`): the API call changes what
  the page shows; a token without the scope gets 403 `insufficient_scope`; a validation
  error comes back as problem+json with `errors[].field`.
- Then `verify`, `./check.sh`, and the `convention-reviewer` agent.
