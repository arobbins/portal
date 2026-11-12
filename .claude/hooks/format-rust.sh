#!/usr/bin/env bash
set -uo pipefail

file=$(jq -r '.tool_input.file_path // .tool_response.filePath // empty')
[[ "$file" == *.rs ]] || exit 0
[[ -f "$file" ]] || exit 0

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

if ! output=$(topcoat fmt --rustfmt "$file" 2>&1); then
  printf 'topcoat fmt failed on %s:\n%s\n' "$file" "$output" >&2
  exit 2
fi
