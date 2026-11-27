#!/usr/bin/env bash
set -euo pipefail

worktree="$(cd "${1:?usage: scripts/check-worktree.sh <worktree path> [check-summary args]}" && pwd)"
shift
cd "$worktree/e2e"
url=$(node --experimental-strip-types --no-warnings -e 'import("./support/environment.ts").then(m => process.stdout.write(m.testDatabaseUrl())).catch(() => process.exit(1))') || url=""
case "$url" in
  *_test) ;;
  *) echo "Couldn't work out the worktree's test database (set TEST_DATABASE_URL)" >&2; exit 1 ;;
esac
cd "$worktree"
export DATABASE_URL="$url"
sqlx database create
sqlx migrate run
scripts/check-summary.sh "$@"
