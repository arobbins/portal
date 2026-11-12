#!/usr/bin/env bash
set -uo pipefail

input=$(cat)
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

state_dir=target/claude-check
mkdir -p "$state_dir"
stamp="$state_dir/last-pass"

fingerprint=$(
  {
    git diff HEAD -- '*.rs' 'Cargo.toml' 'Cargo.lock' 'migrations/*'
    git ls-files --others --exclude-standard -- '*.rs' 'migrations/*' | xargs -I{} shasum {} 2>/dev/null
  } | shasum | cut -d' ' -f1
)

[[ -f "$stamp" && "$(cat "$stamp")" == "$fingerprint" ]] && exit 0

if output=$(CARGO_TARGET_DIR="$state_dir" cargo clippy --all-targets --message-format short --quiet -- -D warnings 2>&1); then
  CARGO_TARGET_DIR="$state_dir" cargo sqlx prepare -- --all-targets >/dev/null 2>&1
  echo "$fingerprint" > "$stamp"
  exit 0
fi

errors=$(printf '%s\n' "$output" | grep -E '^(error|warning)|^[^ ].*: (error|warning)' | head -40)
[[ -n "$errors" ]] || errors=$(printf '%s\n' "$output" | tail -40)

if [[ "$(jq -r '.stop_hook_active // false' <<<"$input")" == "true" ]]; then
  jq -n --arg e "$errors" '{systemMessage: ("cargo clippy still fails:\n" + $e)}'
  exit 0
fi

printf 'cargo clippy (-D warnings) fails. Fix these before finishing:\n%s\n' "$errors" >&2
exit 2
