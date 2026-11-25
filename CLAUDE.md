# Webserver

Rust SaaS starter: axum (server, sessions, `/assets`) hands every other
request to a [Topcoat](https://github.com/tokio-rs/topcoat) 0.12.1 router.
Pages render on the server; interactivity is `$(...)` compiled to JS.
Postgres via sqlx, tower-sessions, argon2, Lightning CSS (`build.rs`).
Area rules in `.claude/rules/` load with matching files; use existing
components (`components.md`) before new markup. Rule files are one topic,
under 4 KB, with `paths` naming only the files they govern (no `src/**`,
`migrations/**` or `e2e/tests/**` outside `layout.md`/`topcoat.md`); split
rather than grow one (`check_conventions.py` enforces it).

## How to work

1. **Test first.** Write the failing test (Rust next to the action, or a
   Playwright spec) and watch it fail for the right reason. Skip only for
   pure styling, and say so.
2. **Use the matching skill** (`new-action`, `new-form-flow`, `new-query`,
   `new-component`, `new-flash`); smallest change that passes.
3. **`verify` each unit** before the next. Fix causes, not symptoms.
4. **Done** = `check --changed` passes (no flag for infrastructure or
   security work; CI runs the full set after push) and
   `convention-reviewer` has no must-fix.
   Commit small, on `main` (a branch only for multi-day work), only when
   asked; "commit" means the `commit` skill: stage, commit, push, watch CI.

## Architecture

- Every feature is one action (`<feature>/actions.rs`) shared by the page,
  `/api/v1` and `/mcp`. It checks `authorize!(caller.require(Permission::X))`
  and logs an account event for every write. Contract: `api.md`.
- Features own their `queries.rs`; others call what the feature exposes.
  Each sqlx query is its own `query_<verb>_<thing>`.
- Errors are `ActionProblem` enums (code, message, field, status); wrap
  shared ones, never copy.
- No hard-coded paths: `href!`, `AccountTab.path()`, `src/routes.rs`.
  Account POSTs are `/account/<thing>/<verb>`.
- Slow or fallible work is a job; shared state lives in Postgres.
- Public ids (`tok_…`, `pk_…`) for anything a browser or API can name.
- Variants and 3+-value state are enums; new state gets a row in
  `.claude/state-machines.md`.
- Config is parsed once in `src/config.rs`; nothing else reads env vars.
- Actions take `env: &Env` (never `cx`/session); time only from `env.now()`.
  Infrastructure never imports a feature. Sizes and boundaries are enforced
  by `scripts/clean_code.py` and `check_conventions.py`.
- Wording: sentence case, buttons "Verb noun" (a modal's heading repeats
  the button that opens it), "Sign in" / "Register"; passkeys are
  *removed*, tokens *revoked*.
- No code comments; explanations go in chat or the rule files.

## Security invariants (never break)

- Password change or sign-out-everywhere bumps `session_version`.
- Account state only from `user.status()`; new users have no password.
- Passwords only via `password_matches`; emails via `utils::parse_email`
  first. Never reveal whether an account exists.
- Emailed-link GETs never consume tokens; tokens are hashed, single use.
- Signals and procedure/shard args are user input and bypass page guards:
  re-check auth.
- Never hold a DB connection across a network call.
- API tokens: hash only, `Authorization: Bearer` only, own audience only,
  never in a flash/session/URL/log, never change security settings.

## Commands

- Run: `topcoat dev` (:3000). E2E: `cd e2e && npm test` (:3100, own DB).
- rtk leaves `cargo test`, Playwright and `git diff` unfiltered (its
  `exclude_commands`), so run them plainly; never add `rtk proxy`.
- Dev database (read-only, secret columns hidden): `scripts/sql.sh "SELECT …"`.
  Writes go through the app or a migration, never this.
- Checks: `scripts/check-summary.sh --changed` (Chromium, only the specs for
  changed areas) while iterating and before commit; no flag for
  infrastructure or security work. Failed Playwright tests are rerun once
  and reported as flaky or environment.
- State machine report: `./check.sh` writes `target/state-machines/index.html`
  (how to model one: `.claude/state-machines.md`).
- Format: `topcoat fmt --rustfmt $(find src -name "*.rs")`, never `leptosfmt`.
- Migrations: always a new file; e2e DB first, then `sqlx migrate run`.

If the setup itself cost time, end with one "Workflow suggestion" line;
change it only once the user agrees.
