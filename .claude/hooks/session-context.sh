#!/usr/bin/env bash
set -uo pipefail

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

with_timeout() {
  perl -e 'alarm shift; exec @ARGV' "$@"
}

branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)
changed=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
echo "Project status at session start:"
echo "- Git: branch $branch, $changed changed file(s)"

migrations=$(with_timeout 8 sqlx migrate info 2>&1)
if [[ $? -eq 0 ]]; then
  pending=$(grep -c '/pending' <<<"$migrations")
  if [[ "$pending" -gt 0 ]]; then
    echo "- Dev database: reachable, $pending pending migration(s) — run \`sqlx migrate run\` before building:"
    grep '/pending' <<<"$migrations" | sed 's/^/    /'
  else
    echo "- Dev database: reachable, all migrations applied"
  fi
else
  echo "- Dev database: NOT reachable (sqlx migrate info failed). Builds still work with SQLX_OFFLINE=true (uses .sqlx/), but new or changed queries need Postgres; ask the user to start it."
fi

if lsof -nP -iTCP:3000 -sTCP:LISTEN >/dev/null 2>&1; then
  echo "- topcoat dev: something is listening on :3000 (don't start a second server; use http://localhost:3000)"
else
  echo "- topcoat dev: nothing on :3000 (start it with \`topcoat dev\` if you need the app running)"
fi

ci_line() {
  with_timeout 5 gh run list --workflow check.yml --limit 1 "$@" \
    --json conclusion,status,headSha,event,createdAt,url \
    -q '.[0] // empty | "\(.status)/\(.conclusion) on \(.headSha[0:7]) (\(.event), \(.createdAt[0:16])Z) \(.url)"' 2>/dev/null
}
if command -v gh >/dev/null 2>&1; then
  ci_dir=$(mktemp -d)
  ci_line --branch main --event push >"$ci_dir/latest" &
  ci_line --event schedule >"$ci_dir/nightly" &
  wait
  latest=$(cat "$ci_dir/latest")
  nightly=$(cat "$ci_dir/nightly")
  rm -rf "$ci_dir"
  echo "- CI (latest push to main): ${latest:-unknown}"
  echo "- CI (latest nightly, all browsers): ${nightly:-none yet}"
  if grep -Eq '/(failure|timed_out|startup_failure)' <<<"$latest$nightly"; then
    echo "  A CI run is red: tell the user before starting new work and offer to fix it (gh run view <id> --log-failed)."
  fi
else
  echo "- CI: gh not installed, status unknown"
fi
exit 0
