#!/usr/bin/env bash
set -uo pipefail

input=$(cat)
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
tool=$(jq -r '.tool_name // empty' <<<"$input")

block() {
  printf '%s\n' "$1" >&2
  exit 2
}

if [[ "$tool" == "Bash" ]]; then
  command=$(jq -r '.tool_input.command // empty' <<<"$input")
  if grep -Eq '(^|[[:space:]/=<"'"'"'])\.env([[:space:];|&<>)"'"'"']|$)' <<<"$command"; then
    block "Blocked: .env holds secrets and is off-limits. Use .env.example for the list of settings; tools like sqlx and cargo read .env themselves."
  fi
  exit 0
fi

file=$(jq -r '.tool_input.file_path // .tool_input.notebook_path // empty' <<<"$input")
[[ -n "$file" ]] || exit 0
rel="${file#"$root"/}"

case "$rel" in
  .env | */.env)
    block "Blocked: .env holds secrets and is off-limits. Edit .env.example instead and tell the user what to add to .env."
    ;;
  assets/dist/* | target/* | e2e/.mail/* | e2e/test-results/* | e2e/playwright-report/*)
    if [[ "$tool" != "Read" ]]; then
      block "Blocked: $rel is generated output. Change the source instead (assets/css/, src/, e2e/tests/)."
    fi
    ;;
  migrations/*)
    if [[ "$tool" != "Read" ]] && git -C "$root" ls-files --error-unmatch -- "$rel" >/dev/null 2>&1; then
      block "Blocked: $rel is a committed migration, which is probably applied already; sqlx checksums applied migrations, so editing one breaks every database. Add a new migration file instead."
    fi
    ;;
esac
exit 0
