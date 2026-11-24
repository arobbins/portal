#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

part=all
changed=0
e2e_args=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --changed) changed=1 ;;
    --part) [[ $# -ge 2 ]] || { echo "--part needs rust or e2e" >&2; exit 2; }; part="$2"; shift ;;
    --) shift; e2e_args+=("$@"); break ;;
    *) echo "Unknown argument: $1 (use --changed, --part rust|e2e (not --part=), -- <playwright args>)" >&2; exit 2 ;;
  esac
  shift
done

case "$part" in
  all | rust | e2e) ;;
  *) echo "--part must be rust or e2e" >&2; exit 2 ;;
esac

if [[ $changed == 1 && "$part" != rust ]]; then
  specs=$(python3 scripts/changed_specs.py)
  if [[ "$specs" == "ALL" ]]; then
    e2e_args+=(--project=chromium)
  elif [[ -z "$specs" ]]; then
    e2e_args+=(--project=chromium --pass-with-no-tests --grep-invert .)
  else
    read -ra changed_args <<<"--project=chromium $specs"
    e2e_args+=("${changed_args[@]}")
  fi
fi

step() {
  printf '\n==> %s\n' "$1"
}

if [[ "$part" != e2e ]]; then
  step "Formatting (cargo fmt + topcoat fmt)"
  cargo fmt --check
  topcoat fmt --check $(find src -name "*.rs")

  step "Project conventions (hard-coded paths, limits in messages)"
  python3 scripts/check_conventions.py

  step "Clippy (warnings are errors)"
  cargo clippy --all-targets -- -D warnings

  step "sqlx offline query data (regenerated)"
  scripts/sqlx-prepare.sh
fi

if [[ "$part" != rust ]]; then
  mkdir -p target
  CARGO_TARGET_DIR=target/e2e topcoat asset bundle >target/e2e-build.log 2>&1 &
  e2e_build=$!
  trap 'kill "$e2e_build" 2>/dev/null || true' EXIT
fi

if [[ "$part" != e2e ]]; then
  step "Rust tests"
  topcoat asset bundle
  rm -f target/state-machines/*.json
  cargo test || { python3 scripts/state_machine_report.py; exit 1; }

  step "State machine report"
  python3 scripts/state_machine_report.py
fi

if [[ "$part" != rust ]]; then
  step "Building the e2e server"
  if ! wait "$e2e_build"; then
    cat target/e2e-build.log
    exit 1
  fi

  step "Behavior tests (Playwright) ${e2e_args[*]:-}"
  (cd e2e && npx playwright test ${e2e_args[@]+"${e2e_args[@]}"}) || { python3 scripts/state_machine_report.py; exit 1; }

  step "State machine report (with browser results)"
  python3 scripts/state_machine_report.py
fi

printf '\nAll checks passed.\n'
