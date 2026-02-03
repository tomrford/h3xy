#!/usr/bin/env bash
set -u
set -o pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [ ! -d "$ROOT_DIR/validation" ]; then
  echo "Missing validation directory: $ROOT_DIR/validation" >&2
  exit 1
fi

VALIDATION_ARGS=${VALIDATION_ARGS:-}

if ! command -v uv >/dev/null 2>&1; then
  echo "Missing uv (required for validation)." >&2
  exit 1
fi

if [ -z "${XDG_CACHE_HOME:-}" ]; then
  CACHE_BASE="${TMPDIR:-/tmp}"
  export XDG_CACHE_HOME="$CACHE_BASE/h3xy-cache"
fi
export UV_CACHE_DIR="${UV_CACHE_DIR:-$XDG_CACHE_HOME/uv}"

if [ -z "$VALIDATION_ARGS" ]; then
  SCRATCHPAD="${SCRATCHPAD:-$ROOT_DIR/scratchpad}"
  if [ -e "$SCRATCHPAD" ]; then
    SCRATCHPAD_REAL="$(readlink -f "$SCRATCHPAD")"
    VALIDATION_ARGS="--inputs-dir $SCRATCHPAD_REAL/validation_inputs --outputs-dir $SCRATCHPAD_REAL/validation_outputs --compare-scratchpad --compare-copy-inputs"
  else
    VALIDATION_ARGS="--compare-copy-inputs"
  fi
fi

cd "$ROOT_DIR/validation"
uv run python main.py $VALIDATION_ARGS
