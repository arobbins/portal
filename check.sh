#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

e2e_args=()
if [[ "${1:-}" == "--changed" ]]; then
  specs=$(python3 scripts/changed_specs.py)
  if [[ "$specs" == "ALL" ]]; then
    e2e_args=(--project=chromium)
  elif [[ -z "$specs" ]]; then
    e2e_args=(--project=chromium --pass-with-no-tests --grep-invert .)
  else
    read -ra e2e_args <<<"--project=chromium $specs"
  fi
fi

step() {
  printf '\n==> %s\n' "$1"
}

step "Formatting (cargo fmt + topcoat fmt)"
cargo fmt --check
topcoat fmt --check $(find src -name "*.rs")

step "Project conventions (hard-coded paths, limits in messages)"
python3 scripts/check_conventions.py

step "Clippy (warnings are errors)"
cargo clippy --all-targets -- -D warnings

step "sqlx offline query data (regenerated)"
scripts/sqlx-prepare.sh

step "Rust tests (building the e2e server alongside)"
CARGO_TARGET_DIR=target/e2e topcoat asset bundle >target/e2e-build.log 2>&1 &
e2e_build=$!
trap 'kill "$e2e_build" 2>/dev/null || true' EXIT
topcoat asset bundle
rm -f target/state-machines/*.json
cargo test || { python3 scripts/state_machine_report.py; exit 1; }

step "State machine report"
python3 scripts/state_machine_report.py

if ! wait "$e2e_build"; then
  cat target/e2e-build.log
  exit 1
fi

step "Behavior tests (Playwright) ${e2e_args[*]:-}"
(cd e2e && npx playwright test ${e2e_args[@]+"${e2e_args[@]}"}) || { python3 scripts/state_machine_report.py; exit 1; }

step "State machine report (with browser results)"
python3 scripts/state_machine_report.py

printf '\nAll checks passed.\n'
