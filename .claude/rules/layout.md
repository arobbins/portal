---
paths:
  - "src/**"
---

# Where things live (the parts the code doesn't make obvious)

- `src/server.rs` is the composition root: it builds `App`, registers job
  handlers in `job_handlers()` and starts workers. Features plug in there;
  `web/middleware.rs` (`inject_globals`) is the one place infrastructure may
  import a feature.
- `src/app.rs` writes `unknown_page` out by hand instead of using
  `not_found!`, because `not_found!` can't take `with(...)` (its own title).
- `src/seed.rs` (`webserver seed`) refuses any database not named `*_test`.
- `src/health.rs` (`/healthz`, `/readyz`) is plain axum, outside sessions and
  the canonical-host redirect.
- `src/api/` is HTTP plumbing only; its endpoint catalog (`catalog.rs`)
  generates the `/docs/api` page, `/api/v1/openapi.json` and `/llms.txt`.
- `src/routes.rs` holds the only cross-feature path constants.

## Feature modules

`src/<feature>.rs` + `src/<feature>/`: `actions.rs`, `queries.rs`, `pages/`,
`emails.rs`. Features today: `auth` (sign-in, sign-up, emailed links, password,
session), `passkeys`, `profile`, `api_tokens`, `devices` (signed-in
browsers), `activity` (the account event log as a list), `admin` (the admin
area: users list and detail, `webserver admin` command), `billing` (plans, Checkout, Stripe webhooks; `src/stripe.rs` is its HTTP client), `account` (the
account shell: sections, pages, cards, nav, the sign-in and devices pages). `user/` is only the `User` model, its
queries, `name.rs` and the event log; `api/` is only HTTP plumbing (auth,
bodies, problems, scopes, catalog, `v1` routes).
