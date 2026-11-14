---
name: new-action
description: Add something the app can do as one shared action that the UI, the JSON API and (later) MCP tools all call — input/output types with JSON Schema, a problem enum with codes, the scope check, the event log, the page wiring, the /api/v1 route and tests. Use whenever adding or changing a feature that reads or changes user data.
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

- Which `ApiScope` does it need? Reuse `<area>:read` / `<area>:write`, or add a pair
  (enum variant with `#[serde(rename = "area:read")]`, `ALL`, `label`, `choice_label`,
  `implies`, and a migration extending the `api_tokens.scopes` CHECK). No wildcard scopes.
- Security-sensitive (password, email, passkeys, tokens, deleting the account, billing
  ownership)? Then it's `caller.require_browser()` and gets **no** API route.
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
    authorize!(caller.require(ApiScope::ThingWrite));
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

## 6. API route in `src/api/v1/<feature>.rs`

```rust
#[route(GET "/api/v1/things")]
pub async fn list_things_route(cx: &Cx) -> Result<ApiResponse> {
    api_call(cx, async |caller| Ok(respond(list_things(cx, &caller).await?))).await
}

#[route(POST "/api/v1/things", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn create_thing_route(cx: &Cx, body: Body) -> Result<ApiResponse> {
    api_call_with_body(cx, body, async |caller, input: DoThing| {
        Ok(respond(do_thing(cx, &caller, &input).await?))
    })
    .await
}
```

Then document it: add an `Endpoint` to `api::catalog::endpoints()` (id `<verb>-<thing>`,
summary "Verb your thing", scope, request and response `schema_for!`, examples built from
real values, errors from the problem enum's `ALL` plus `ApiFailure::InsufficientScope` and
`BodyProblem::ALL` for bodies). The docs page, the OpenAPI document and the "every
documented endpoint exists" test pick it up. Add `pub mod <feature>;` to `src/api/v1.rs`.

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
