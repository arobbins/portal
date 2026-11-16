#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
RUST_LOG=error cargo run --quiet -- sql "$@"
