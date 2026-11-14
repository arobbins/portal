#!/usr/bin/env bash
set -uo pipefail

cd "$(dirname "$0")/.."
log=target/check.log
mkdir -p target
rm -rf e2e/test-results

./check.sh >"$log" 2>&1
status=$?

if [[ $status -eq 0 ]]; then
  tests=$(grep -Eo '[0-9]+ passed' "$log" | tr '\n' ' ')
  echo "All checks passed (${tests% }). Full log: $log"
  exit 0
fi

step=$(grep '^==> ' "$log" | tail -1 | sed 's/^==> //')
echo "FAILED at: $step (full log: $log)"
echo

if [[ "$step" == Behavior* ]]; then
  grep -E '^\s+[0-9]+\) \[' "$log" | sed 's/ ─*$//' | sort -u | head -20
  echo
  for context in e2e/test-results/*/error-context.md; do
    [[ -f "$context" ]] || continue
    echo "--- $(dirname "$context")"
    awk '/^# Error details/{on=1; next} /^# Page snapshot/{on=0} on' "$context" | grep -v '^\s*$' | head -15
    ls "$(dirname "$context")"/*.png 2>/dev/null | head -1 | sed 's/^/screenshot: /'
  done | head -150
else
  section=$(sed -n "/^==> $step/,\$p" "$log")
  errors=$(grep -E '^(error|warning)|-->|^\s+\||panicked|FAILED|failed|Convention|^  src/|left:|right:|^Diff in|would reformat|need formatting' <<<"$section" | head -60)
  if [[ -n "$errors" ]]; then
    printf '%s\n' "$errors"
  else
    tail -30 <<<"$section"
  fi
fi
exit "$status"
