#!/usr/bin/env bash
set -u
set -o pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
RALPH_DIR="$ROOT_DIR/ralph"
STATUS_FILE="$RALPH_DIR/status.md"
PROMPT_FILE="$RALPH_DIR/prompt.md"
RESULT_FILE="$RALPH_DIR/results/latest_result.md"
HISTORY_DIR="$RALPH_DIR/history"
VALIDATION_RUNNER="$ROOT_DIR/scripts/run_validation.sh"

MAX_ITERATIONS="${MAX_ITERATIONS:-200}"
SLEEP_SECONDS="${SLEEP_SECONDS:-2}"
SKIP_VALIDATION="${SKIP_VALIDATION:-0}"

mkdir -p "$RALPH_DIR/results" "$HISTORY_DIR"

if [ ! -f "$STATUS_FILE" ]; then
  {
    echo "Status: running"
    echo ""
    echo "Memory:"
    echo "-" 
    echo ""
    echo "Notes:"
    echo "-"
  } > "$STATUS_FILE"
fi

if [ ! -f "$PROMPT_FILE" ]; then
  echo "Missing prompt file: $PROMPT_FILE" >&2
  exit 1
fi

iteration=0
echo "Starting ralph loop..."

while true; do
  iteration=$((iteration + 1))
  if [ "$iteration" -gt "$MAX_ITERATIONS" ]; then
    echo "Max iterations ($MAX_ITERATIONS) reached. Stopping."
    break
  fi

  status="$(awk -F': ' '/^Status:/{print $2; exit}' "$STATUS_FILE" | tr -d '\r')"
  if [ "$status" = "done" ] || [ "$status" = "blocked" ]; then
    echo "Agent stopped with status: $status"
    cat "$STATUS_FILE"
    break
  fi

  echo ""
  echo "=== Running iteration $iteration/$MAX_ITERATIONS at $(date) ==="
  echo "Current status: ${status:-unknown}"

  if [ -x "$VALIDATION_RUNNER" ]; then
    SKIP_VALIDATION="$SKIP_VALIDATION" "$VALIDATION_RUNNER" || true
  else
    echo "Missing validation runner: $VALIDATION_RUNNER" >&2
  fi

  prompt="$(cat "$PROMPT_FILE")"
  run_log="$HISTORY_DIR/iteration_${iteration}_$(date +%Y%m%d_%H%M%S).log"
  codex exec "$prompt" --model gpt-5.2-codex --full-auto --config model_reasoning_effort="xhigh" | tee "$run_log"

  if [ -f "$RESULT_FILE" ]; then
    echo "Latest results written to $RESULT_FILE"
  fi

  sleep "$SLEEP_SECONDS"
done

echo ""
echo "Loop completed."
