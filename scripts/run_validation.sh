#!/usr/bin/env bash
set -u
set -o pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
LOG_DIR="$ROOT_DIR/ralph/results"
mkdir -p "$LOG_DIR"

run_id="$(date +%Y%m%d_%H%M%S)"
timestamp="$(date +%Y-%m-%dT%H:%M:%S%z)"
report="$LOG_DIR/result_${run_id}.md"
latest="$LOG_DIR/latest_result.md"

test_status=0
validation_status=0

{
  echo "# Validation Run"
  echo ""
  echo "- Timestamp: $timestamp"
  echo "- Workspace: $ROOT_DIR"
  echo ""
  echo "## Test Suite"
  echo ""
  echo "### cargo test"
  echo '```'
} > "$report"

cargo test 2>&1 >> "$report"
test_status=$?

{
  echo '```'
  echo ""
  echo "- Exit: $test_status"
  echo ""
  echo "## Validation Suite"
  echo ""
} >> "$report"

validation_cmd=""
if [ -n "${VALIDATION_CMD:-}" ]; then
  validation_cmd="$VALIDATION_CMD"
elif [ -x "$ROOT_DIR/scripts/validation_suite.sh" ]; then
  validation_cmd="$ROOT_DIR/scripts/validation_suite.sh"
elif [ -x "$ROOT_DIR/scripts/validate.sh" ]; then
  validation_cmd="$ROOT_DIR/scripts/validate.sh"
fi

if [ -n "$validation_cmd" ]; then
  {
    echo "### $validation_cmd"
    echo '```'
  } >> "$report"
  bash -lc "$validation_cmd" 2>&1 >> "$report"
  validation_status=$?
  {
    echo '```'
    echo ""
    echo "- Exit: $validation_status"
  } >> "$report"
else
  validation_status=127
  {
    echo "### (missing validation command)"
    echo ""
    echo "Set \\`VALIDATION_CMD\\` or add \\`scripts/validation_suite.sh\\`."
    echo ""
    echo "- Exit: 127"
  } >> "$report"
fi

cp "$report" "$latest"

if [ "$test_status" -ne 0 ] || [ "$validation_status" -ne 0 ]; then
  exit 1
fi

exit 0
