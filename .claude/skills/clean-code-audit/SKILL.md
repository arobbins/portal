---
name: clean-code-audit
description: Run a repeatable clean-code audit of this codebase — measure function/file sizes and boundary violations, review every module against the clean-code checklist (single responsibility, injected dependencies, feature boundaries, typed data, duplication, testability), score it out of 100, fix findings in priority order, verify, and ratchet the automated limits tighter. Use when asked for a code-quality audit, a cleanup pass, or "make the codebase cleaner"; safe to run again and again.
---

# Clean-code audit

Each run leaves the codebase measurably cleaner and the checks stricter, so
it never slides back. Work in this order.

## 1. Measure

```sh
python3 scripts/clean_code.py --report
```

It prints the function-size distribution (plain functions vs components and
pages), the largest functions and files (lines before tests), and every
automated violation. Record the numbers in the report at the end; they are
the baseline for the next run.

The automated rules (`scripts/clean_code.py`, which `check_conventions.py`,
`./check.sh` and the Stop hook all run) are:

| Rule | Limit / fix |
|---|---|
| Plain function length | ≤ `MAX_FUNCTION_LINES` (40): extract named helpers |
| Component / page length | ≤ `MAX_VIEW_LINES` (80): extract child components |
| File length before tests | ≤ `MAX_FILE_LINES` (300): split by responsibility (`token_routes.rs`, `token_fields.rs`, `password_routes.rs` are the pattern) |
| Actions take `cx` or the session | take `env: &Env` + `Caller`; ceremony state goes behind a trait (`PasskeySession`) |
| `now_utc()` outside `env.rs` | `env.now()` / `app.clock.now()` (tests use `Clock::Fixed`) |
| `context::<…>` outside `web/` | `web::app(cx)`, `env(cx)`, `config(cx)` |
| `rewrite(` outside `web/context.rs` | `web::redisplay(path, failure)` |
| `#[allow(dead_code)]` | delete it, or `#[cfg(test)]` |
| `pub mod queries` in a feature | keep private; expose a function (`api_tokens::lifecycle::revoke_all_for_user`) |
| Infrastructure (`action`, `env`, `jobs`, `mail`, `rate_limit`, `web`, …) importing a feature | invert: a trait, a `Job`, or registration in `server.rs` (the composition root). `web/middleware.rs` is the one allowed composition point |

## 2. Review what scripts can't see

Read every module changed since the last audit (`git log` / `git diff
main --stat`), plus the five largest files from the report. For each, check:

1. **Single responsibility**: can you describe the file in one sentence
   without "and"? Pages compose components; routes and procedures sit in
   `*_routes.rs`; form fields in `*_fields.rs`; view models (`TokenRow::new`)
   in `*_view.rs`.
2. **Dependencies are injected**: actions get `&Env` (db, limits, config,
   clock, ip), jobs get `&App`, nothing reaches for globals. A function that
   needs one value takes that value, not `cx`.
3. **Feature boundaries**: no feature calls another feature's `query_*`;
   cross-feature links go through `routes.rs`; infrastructure knows no
   feature (handlers are registered in `server::job_handlers()`).
4. **No duplication**: a pattern written twice becomes a helper. Known
   shared pieces to reuse: `DialogForm` + `dialog_input` /
   `dialog_form_error` / `dialog_error_toast` (modal forms),
   `EmailTemplate` (every email), `ProblemSpec::new(..).on_field(..)`
   (problems), `redisplay` (form failures), `NewLink` + `email_user_link`
   (emailed links), `Handler::of::<J>()` (jobs), `ApiAction` (API, docs,
   MCP).
5. **Typed data**: no `String` where an enum exists (scopes, audiences,
   kinds, statuses); DB rows map to typed models (`ApiTokenRow` →
   `ApiToken`); errors are enums or a named error type (`JobError`,
   `MailError`), never `String`.
6. **Testability**: logic that depends on time takes `now`; logic that
   depends on the session takes a trait; pure logic has a unit test next to
   it; helpers for tests live in `testing.rs` or `#[cfg(test)]`.
7. **Names**: modules named for what they hold (`lifecycle`, not `verify`);
   functions are verbs; no `utils` grab bags growing new members.
8. **Encapsulation**: the narrowest visibility that compiles
   (`pub(super)` inside `pages/`, private `queries`); `pub` only for what
   another module uses.
9. **Project conventions**: CLAUDE.md, `.claude/rules/*.md` (error
   contract, `authorize!`, events in the same transaction, public ids,
   wording, BEM/CSS scales).

## 3. Score

Start at 100 and subtract, then report the score with the previous run's
score (from the last audit's report in the conversation or git log) and the
measurements from step 1:

- automated violation: −3 each
- must-fix review finding (boundary broken, security invariant, duplicated
  logic, service locator, stringly-typed state): −4 each
- should-fix (naming, visibility, a function near its limit, a missing
  test): −1 each, at most −15 total

## 4. Fix, in priority order

1. Security invariants and boundary violations.
2. Duplication (extract the shared piece once; update every copy).
3. Injected dependencies and testability.
4. Size: split files and functions (never by line count alone — split
   along responsibilities).
5. Types, names, visibility.

Ask the user only for decisions that change behaviour or public URLs.
Keep every change behaviour-preserving; move tests with the code they test.

## 5. Verify

1. `topcoat fmt --rustfmt $(find src -name "*.rs")`
2. `SQLX_OFFLINE=false cargo clippy --all-targets --target-dir target/claude-check -- -D warnings`
3. The `check` skill (full `./check.sh`).
4. `convention-reviewer` agent on the diff; fix its must-fix findings.

## 6. Ratchet

When the report shows headroom (the largest plain function well under the
limit, no file near 300), lower the constant at the top of
`scripts/clean_code.py` to just above the current maximum. Turn any new
review finding that a script could catch into a rule in
`scripts/clean_code.py` (with its fix in the message) and a row in the
table above. Update CLAUDE.md / `.claude/rules/` when a pattern changed.

End with: score (before → after), the measurements, what was fixed, what
was deferred and why, and any ratchet applied.
