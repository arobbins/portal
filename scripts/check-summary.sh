#!/usr/bin/env bash
set -uo pipefail

cd "$(dirname "$0")/.."
log=target/check.log
mkdir -p target
rm -rf e2e/test-results

./check.sh "$@" >"$log" 2>&1
status=$?

environment='browserType.launch: Timeout|Timed out waiting [0-9]+ms from config.webServer|EADDRINUSE|Process from config.webServer exited early'
step=$(grep '^==> ' "$log" | tail -1 | sed 's/^==> //')

if [[ $status -ne 0 && "$step" == Behavior* ]]; then
  first_failures=$(grep -E '^\s+[0-9]+\) \[' "$log" | sed 's/ ─*$//' | sort -u)
  cause="flaky"
  grep -Eq "$environment" "$log" && cause="environment (load average: $(sysctl -n vm.loadavg 2>/dev/null || cat /proc/loadavg 2>/dev/null))"
  printf '\n==> Rerunning failed Playwright tests once\n' >>"$log"
  (cd e2e && npx playwright test --last-failed) >>"$log" 2>&1
  if [[ $? -eq 0 ]]; then
    echo "All checks passed after one rerun. First-run failures ($cause):"
    printf '%s\n' "${first_failures:-  (none listed: the run failed before tests started)}"
    echo "Full log: $log"
    exit 0
  fi
  grep -Eq "$environment" "$log" && echo "ENVIRONMENT FAILURE (not the code): the browser or test server didn't start. Rerun when the machine is less busy."
fi

if [[ $status -eq 0 ]]; then
  tests=$(grep -Eo '[0-9]+ passed' "$log" | tr '\n' ' ')
  echo "All checks passed (${tests% }). Full log: $log"
  exit 0
fi

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
