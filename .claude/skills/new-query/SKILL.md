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
- Apply it to the dev database: `sqlx migrate run` (sqlx macros check queries against it at
  compile time). The test database is migrated by `webserver seed`; don't touch it by hand.
- If seeded e2e users need the new column, update `src/seed.rs` and `e2e/support/users.ts`
  together.

## 2. Query function

Every query is its own `async fn` in the feature's `queries.rs` (users: `src/user/queries.rs`):

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

- Name starts with `query_`; `db` is always the **last** parameter. Use
  `impl PgExecutor<'_>` when it may run inside a transaction, `&PgPool` otherwise.
- Return `Result<_, sqlx::Error>`; callers turn it into `topcoat::Result` with `?`.
- Inserts with several columns take a param struct (`NewUser<'a> { name: &'a str, ... }`).
- Always use the checked macros (`query!`, `query_as!`, `query_scalar!`), never string SQL.
- Enum columns: select as `role AS "role: Role"`; bind with the enum's `as_str()`.
- Never call sqlx inline in a handler, page or procedure.

## 3. Security checks for the query

- Anything that changes a password, or should sign out every device, sets
  `session_version = session_version + 1`.
- Single-use tokens are consumed with `DELETE ... RETURNING`; only SHA-256 hashes are stored.
- Rows owned by a user are always filtered by `user_id` from `require_user(cx)`, never by an id
  from the browser alone.
- Emails reaching a query have gone through `utils::parse_email`.
- Never hold a DB connection or transaction across a network call (email, HTTP).

## 4. Offline query data

After adding or changing any query, regenerate the committed `.sqlx/` metadata so the app
builds without a database (CI, worktrees, cloud sessions):

```sh
cargo sqlx prepare -- --all-targets
```

The stop hook runs this automatically after clippy passes, and `./check.sh` fails if `.sqlx/`
is stale. Include the changed `.sqlx/*.json` files in the commit.

## 5. Finish

- If new state with more than two values appears, add an enum and a row in the CLAUDE.md
  "State machines" table.
- If cleanup is needed for expiring rows, add it to `src/cleanup.rs`.
- Run `./check.sh`.
