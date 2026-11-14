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

Problems: an enum implementing `ActionProblem` (`code` snake_case, never renamed once
shipped; `message`; `field` naming the input field; `status` if not 422). Wrap shared
problems instead of copying them: `NameProblem`, `PasswordProblem`,
`CurrentPasswordProblem` (via `check_current_password`), `TooManyAttempts`.

## 4. The action

```rust
pub async fn do_thing(cx: &Cx, caller: &Caller, input: &DoThing) -> ActionResult<ThingView, ThingProblem> {
    authorize!(caller.require(Permission::EditThings));
    let name = match check_name(&input.name) {
        Ok(name) => name,
        Err(problem) => return fail(ThingProblem::Name(problem)),
    };
    let mut tx = db(cx).begin().await?;
    // query_* functions only (new-query skill) on &mut *tx, scoped by caller.owner_id()
    record_event(cx, &EventSource::of(caller), AccountEventKind::ThingCreated,
        json!({ "thing": public_id, "name": name }), &mut *tx).await?;
    tx.commit().await?;
    Ok(Ok(view))
}
```

- Every write records an event inside the write's transaction; `detail` is a JSON object
  (ids and names, never secrets). Add the `AccountEventKind` (+ `ALL`) and a migration
  extending the `kind` CHECK.
- Owned rows are scoped by `caller.owner_id()` (the workspace; today the user).
- Send emails or call other services only after `commit`.

## 5. Page wiring

```rust
#[procedure]
async fn do_thing_js(cx: &Cx, name: String) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    match do_thing(cx, &caller, &DoThing { name }).await? {
        Ok(view) => Ok(Ok(view.name)),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/things/create")]
pub async fn create_thing_submit(cx: &Cx, Form(input): Form<DoThing>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    match do_thing(cx, &caller, &input).await? { … }
}
```

POST routes under `/account` are `/account/<thing>/<verb>`; forms for signed-out pages post
to the page's own URL. Redirect targets come from `href!(…)`, `AccountTab::X.href(cx)` or
`AccountSection::X.href(cx)`. The rest follows `new-form-flow`.

## 6. Expose it to the API (scoped actions only)

Implement `ApiAction` next to the action (`profile/actions.rs` is the reference): one
declaration drives the route handler, the `/docs/api` page, the OpenAPI document,
`/llms.txt` and the MCP tool list.

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

    async fn run(cx: &Cx, caller: &Caller, input: &DoThing) -> ActionResult<ThingView, ThingProblem> {
        do_thing(cx, caller, input).await
    }

    fn example_input() -> Option<DoThing> { Some(/* real-looking values */) }
    fn example_output() -> ThingView { /* … */ }
    fn problems() -> Vec<ErrorDoc> { ThingProblem::ALL.into_iter().map(ErrorDoc::of).collect() }
}
```

Add it to `endpoints()` in `src/api/registry.rs`, then the thin route in
`src/api/v1/<feature>.rs` (Topcoat needs the path in the attribute):

```rust
#[route(POST "/api/v1/things", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn create_thing_route(cx: &Cx, body: Body) -> Result<ApiResponse> {
    serve_with_body::<CreateThingAction>(cx, body).await   // `serve::<A>(cx)` for GET
}
```

Add `pub mod <feature>;` to `src/api/v1.rs`. The scope, body and problem errors are
documented automatically; `scripts/check_conventions.py` fails if a `/api/v1` route has no
registered `ApiAction`, and `main.rs` tests that every registered endpoint exists and needs
a token.

Lists: cursor pagination. Creates: accept `Idempotency-Key` (build the shared mechanism
the first time it's needed).

## 7. Tests

- Rust (`#[sqlx::test]`, `let cx = Cx::new().with(db.clone())` calls the action
  directly): the scope matrix (browser ok, token without the scope →
  `Denied::MissingScope`, token with it ok, browser-only actions → `Denied::BrowserOnly`
  with nothing written and no event), a schema test (`schemars::schema_for!`: required
  fields, limits, `additionalProperties: false`), and a `*_matches_the_limit` test for any
  number written into a message.
- e2e (`api-<feature>.spec.ts`, helpers in `support/api.ts`): the API call changes what
  the page shows; a token without the scope gets 403 `insufficient_scope`; a validation
  error comes back as problem+json with `errors[].field`.
- Then `verify`, `./check.sh`, and the `convention-reviewer` agent.
