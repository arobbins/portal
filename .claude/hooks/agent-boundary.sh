#!/usr/bin/env bash
set -uo pipefail

role="${1:-}"
input=$(cat)
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
tool=$(jq -r '.tool_name // empty' <<<"$input")

block() {
  printf '%s\n' "$1" >&2
  exit 2
}

relative() {
  local path="${1#"$root"/}"
  printf '%s' "${path#./}"
}

target=$(relative "$(jq -r '.tool_input.file_path // .tool_input.path // .tool_input.notebook_path // empty' <<<"$input")")
pattern=$(jq -r '.tool_input.pattern // .tool_input.glob // empty' <<<"$input")
command=$(jq -r '.tool_input.command // empty' <<<"$input")

scenario_writer() {
  case "$tool" in
    Read | Grep | Glob)
      [[ "$target" == src/* || "$target" == src || "$pattern" == src/* || "$pattern" == *"/src/"* ]] &&
        block "Blocked: the scenario writer judges the build independently, so it never reads the implementation (src/). Work from the spec, e2e/support/ and the pages as a user sees them."
      [[ "$tool" != "Read" && -z "$target" && "$pattern" != e2e/* && "$pattern" != specs/* ]] &&
        block "Blocked: search with a path under e2e/ or specs/, so the search can't reach src/."
      ;;
    Write | Edit)
      [[ "$target" == e2e/scenarios/* || "$target" == e2e/support/* ]] ||
        block "Blocked: the scenario writer only writes e2e/scenarios/ and e2e/support/."
      ;;
    Bash)
      grep -Eq '(^|[^[:alnum:]_.])src(/|[[:space:]]|$)|\.rs([^[:alnum:]]|$)' <<<"$command" &&
        block "Blocked: the scenario writer never reads the implementation (src/, *.rs)."
      if grep -Eq '(^|[[:space:];|&(])(grep|egrep|rg|ag|find|fd|tree)([[:space:]]|$)' <<<"$command" &&
        ! grep -Eq '(e2e|specs)/' <<<"$command"; then
        block "Blocked: searches must name a path under e2e/ or specs/, so they can't reach src/."
      fi
      ;;
  esac
}

spec_builder() {
  case "$tool" in
    Write | Edit)
      [[ "$target" == e2e/scenarios/* || "$target" == specs/* ]] &&
        block "Blocked: the builder never edits specs or scenarios. If a scenario can't pass as written, stop and report the conflict; the main session drafts a spec amendment."
      ;;
    Read | Grep | Glob)
      [[ "$target" == e2e/scenarios* || "$pattern" == *scenarios* ]] &&
        block "Blocked: the scenarios are a holdout. Build from the spec; run them with 'npx playwright test scenarios/<name>.spec.ts' and read the failure output."
      ;;
    Bash)
      if grep -Eq '(^|[[:space:];|&(])(grep|egrep|rg|ag|find|fd|tree)([[:space:]]|$)' <<<"$command" &&
        ! grep -Eq '(^|[[:space:]"'"'"'=])(\./)?(src|assets|migrations|scripts|\.claude|e2e/tests|e2e/support|specs)(/|[[:space:]"'"'"']|$)' <<<"$command"; then
        block "Blocked: searches must name a path (src/, assets/, migrations/, e2e/tests/, …), so they can't reach the holdout scenarios."
      fi
      if grep -q 'scenarios' <<<"$command" && ! grep -Eq 'playwright test|check-summary\.sh|check_conventions\.py' <<<"$command"; then
        block "Blocked: the scenarios are a holdout. Run them with Playwright or check, but don't read them."
      fi
      grep -Eq '(>|tee|sed -i|mv|cp|rm)[^|;&]*(e2e/scenarios|specs/)' <<<"$command" &&
        block "Blocked: the builder never edits specs or scenarios."
      ;;
  esac
}

case "$role" in
  scenario-writer) scenario_writer ;;
  spec-builder) spec_builder ;;
esac
exit 0
