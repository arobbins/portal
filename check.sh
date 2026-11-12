#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

step() {
  printf '\n==> %s\n' "$1"
}

step "Formatting (cargo fmt + topcoat fmt)"
cargo fmt --check
topcoat fmt --check $(find src -name "*.rs")

step "Clippy (warnings are errors)"
cargo clippy --all-targets -- -D warnings

step "sqlx offline query data is up to date"
cargo sqlx prepare --check -- --all-targets

step "Rust tests"
topcoat asset bundle
cargo test

step "Behavior tests (Playwright)"
(cd e2e && npx playwright test)

printf '\nAll checks passed.\n'
