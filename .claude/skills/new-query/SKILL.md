---
name: new-query
description: Add a database change or query to this app — a sqlx migration in migrations/, then compile-time-checked query functions following the project's query conventions. Use whenever adding a table, column, constraint, or a new SQL query.
---

# New migration / query

## 1. Migration (only if the schema changes)

- File: `migrations/<YYYYMMDDHHMMSS>_<snake_description>.sql`, timestamp later than every
  existing file (`ls migrations`). Plain SQL, no down migration.
- Push rules into the database: `NOT NULL`, `CHECK`s matching Rust enums
  (`CHECK (role IN (...))`), `REFERENCES ... ON DELETE CASCADE`, `UNIQUE`, and an index for each
  foreign key you filter by (see `20261111120000_passkeys.sql`).
- Users table invariants that must survive: `CHECK ((password IS NULL) = (verified_at IS NULL))`.
- Migrations that rewrite existing rows: drop or relax a constraint *before* the `UPDATE`
  that would violate it, then add the new one.
- **Try it on the e2e test database first**: `cd e2e && npx playwright test
  tests/home.spec.ts --project=chromium --reporter=line` (its server runs `sqlx migrate run` against the
  `_test` database, which keeps older rows than dev). If it fails, fix the file freely: it
  isn't applied anywhere that matters yet.
- Only then apply it to the dev database: `sqlx migrate run` (sqlx macros check queries
  against it at compile time). From here on the file is frozen (the hook blocks edits);
  further changes go in a new migration. Don't touch the test database by hand.
- If seeded e2e users need the new column, update `src/seed.rs` and `e2e/support/users.ts`
  together.

## 2. Query function

Every query is its own `async fn` in the feature's `queries.rs` (`src/<feature>/queries.rs`;
the `users` table: `src/user/queries.rs`; emailed links: `src/auth/queries.rs`):

```rust
pub async fn query_<verb>_<thing>(
    arg: Type,
    db: impl PgExecutor<'_>,
) -> Result<ReturnType, sqlx::Error> {
    let value = sqlx::query_scalar!("...", arg)
        .fetch_optional(db)
        .await?;

    Ok(value)
}
```

- Name is `query_<verb>_<thing>[_by_<key>]`; verbs: `get` (one row or `None`), `list`
  (`Vec`), `insert`, `update`, `delete`, `consume` (single-use delete), `record` (usage
  stamps), or a domain verb (`snooze`, `rename`). `db` is always the **last** parameter and
  always `impl PgExecutor<'_>` (works with a pool, a connection or `&mut *tx`).
- Ids are passed by value (`user_id: i32`); ids that came from a browser or API are public
  ids (`public_id: &str`) and are always paired with the owner (`AND user_id = $2`).
- Return the macro's result directly (`.fetch_optional(db).await`) unless you map it.
- Return `Result<_, sqlx::Error>`; callers turn it into `topcoat::Result` with `?`.
- Inserts with several columns take a param struct (`NewUser<'a> { name: &'a str, ... }`).
- Always use the checked macros (`query!`, `query_as!`, `query_scalar!`), never string SQL.
- Enum columns: select as `role AS "role: Role"`; bind with the enum's `as_str()`.
- Never call sqlx inline in a handler, page or procedure.
- **A sort column chosen at runtime** (list pages): keep the checked macro and bind the
  `SortKey::as_str()` value plus a `descending: bool`. Compute derived columns in a `WITH`
  CTE, then order with one `CASE` per type and direction, e.g.
  `CASE WHEN NOT $3 THEN CASE $2 WHEN 'name' THEN lower(name) WHEN 'email' THEN email END END ASC,
  CASE WHEN $3 THEN … END DESC`, a second pair for timestamps with `NULLS LAST`, and a
  stable tiebreak (`created_at DESC, id DESC`) last. `admin/queries.rs` `query_search_users`
  is the reference.

## 3. Security checks for the query

The security invariants in `CLAUDE.md` apply to every query. Query-specific ones:

- Single-use tokens are consumed with `DELETE ... RETURNING`; only SHA-256 hashes are stored.
- Rows owned by a user are always filtered by the caller's `owner_id()`, never by an id from
  the browser alone.
- Tables whose rows a browser or API can name get a `public_id TEXT NOT NULL UNIQUE` with a
  format CHECK (`'^tok_[A-Za-z0-9]{20}$'`) and a `secret::PublicIdKind` variant.
- Event `detail` is a JSON object (`CHECK (jsonb_typeof(detail) = 'object')`).

## 4. Offline query data

After adding or changing any query, regenerate the committed `.sqlx/` metadata so the app
builds without a database (CI, worktrees, cloud sessions):

```sh
scripts/sqlx-prepare.sh
```

It touches `src/main.rs` first: a plain `cargo sqlx prepare` after `topcoat fmt` can reuse
cached macro output and write stale metadata.

The stop hook runs it after clippy passes, and `./check.sh` regenerates `.sqlx/` itself. Include the changed `.sqlx/*.json` files in the commit.

## 5. Finish

- If new state with more than two values appears, add an enum and a row in
  `.claude/state-machines.md`.
- If expiring rows need deleting, expose a `pub async fn delete_*(app: &App)` from the
  feature (it reads `app.clock.now()`) and call it from `cleanup::run` (the hourly
  `CleanupJob`); `cleanup.rs` never calls a feature's `query_*` directly.
- Sending email or calling another service after a write: `enqueue(&SomeJob { … }, &mut *tx)`
  in the same transaction, never inline. `SomeJob` implements `trait Job` (`KIND`, `run`)
  and is listed in `server::job_handlers()`; add its `KIND` to the `jobs` kind CHECK.
- Run `./check.sh`.
