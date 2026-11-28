---
paths:
  - "src/api.rs"
  - "src/api/**"
  - "src/profile/api.rs"
  - "src/pages/api_docs.rs"
  - "src/pages/api_docs/**"
  - "src/pages/llms_txt.rs"
  - "src/server/tests.rs"
  - "src/server/tools.rs"
  - "e2e/tests/api-*.spec.ts"
  - "e2e/support/api.ts"
---

# JSON API and API docs

## JSON API (`src/api/`)

- **Registry** (`api/action.rs` trait, `api/registry.rs` docs, `api/tool.rs`
  MCP tools, `api/serve.rs` routes): every API endpoint is an `ApiAction`
  impl next to its action (id, group, method, path, summary, description,
  permission, `Input`/`Output`/`Problem` types, examples, problem docs) listed
  in `server::tools::tools()`. `Endpoint.scope` is `None` for `AnyToken` actions
  (`send_feedback`): docs use `access_note()` / `scope_name()`. The docs page, `/api/v1/openapi.json`, `/llms.txt` and
  the MCP tools are generated from it; nothing about an endpoint is
  written twice.
- Routes live in `api/v1/<feature>.rs` as Topcoat `#[route]`s named
  `<verb>_<thing>_route` whose whole body is `serve::<A>(cx)` (no body) or
  `serve_with_body::<A>(cx, body)`; they authenticate, read the JSON body,
  run the action and turn server errors into 500 `internal_error`. Public
  endpoints use `public_api_call`. No logic in routes.
- Errors are RFC 9457 `application/problem+json`: `type`, `title`,
  `status`, `detail`, plus `code` and, for field problems, `errors: [{field,
  code, message}]` (`ApiProblem::of(problem)`; extra members with
  `.extra(key, value)`). The browser's own JSON routes (passkeys) use the same
  shape. Bad JSON/unknown fields → 400 `invalid_json`, wrong content type →
  415, bodies over 64 KB → 413 (routes with a body add
  `with(BodyLimit::max(MAX_JSON_BODY))`), unknown `/api/*` paths → 404
  `not_found`. A wrong method gets Topcoat's 405 with an `Allow` header.
  Write `detail` so a model knows how to fix the request.
- Every response is `Cache-Control: no-store`.
- Conventions for when they're first needed: lists use cursor pagination
  (`?cursor=&limit=`, response `{ "data": [...], "next_cursor": … }`);
  POSTs that create something accept an `Idempotency-Key` header; breaking
  changes go to `/api/v2`.

## Docs (`/docs/api`, `/api/v1/openapi.json`, `/llms.txt`)

- Both (and `/llms.txt`) are generated from the registry: `endpoint::<A>()`
  turns each `ApiAction` into an `Endpoint` (id `<verb>-<thing>`, summary
  "Verb your thing", `schema_for!` request/response, examples built from
  real values, the scope error, body errors for methods with a body, and the
  action's problems). Every error response in the OpenAPI document carries a
  real problem+json example per code. Field tables read the schemars
  descriptions and length limits, so write good `#[schemars(description)]`s.
  Never hand-write an error code or field list on the page.
- Adding an endpoint means an `ApiAction` impl plus a line in
  `server/tools.rs`; `server/tests.rs` tests that every catalog entry exists and needs a token, and
  `api-docs.spec.ts` checks the page and the OpenAPI document.
- Error enums the docs list need `pub const ALL` (`ApiAuthProblem`,
  `BodyProblem`, `NameProblem`); an action's `Problem` implements
  `ListedProblem` (`all()`) for the docs; shared API errors are `ApiFailure`.
