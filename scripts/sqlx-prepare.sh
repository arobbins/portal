#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

touch src/main.rs
cargo sqlx prepare -- --all-targets
