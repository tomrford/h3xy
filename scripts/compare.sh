#!/usr/bin/env bash
#
# compare.sh - Compare h3xy (cargo) output against HexView.exe reference
#
# Usage:
#   ./scripts/compare.sh [OPTIONS] -- <hexview_args>
#
# Examples:
#   ./scripts/compare.sh -- input.hex /AR:'0x1000-0x2FFF' -o output.hex
#   ./scripts/compare.sh -v -- input.hex /FR:'0x1000,0x100' /FP:FF -o output.hex
#   ./scripts/compare.sh -k -- input.hex -o output.hex  # keep output files
#
# Options:
#   -v, --verbose     Show detailed output
#   -k, --keep        Keep output files after comparison
#   -s, --scratchpad  Use scratchpad directory for Windows paths (WSL mode)
#   -h, --help        Show this help message
#
# Environment:
#   HEXVIEW_EXE       Path to HexView.exe (default: ./reference/HexView.exe)
#   SCRATCHPAD        Path to scratchpad directory (default: ./scratchpad)
#
# Exit codes:
#   0 - Outputs match
#   1 - Outputs differ
#   2 - Execution error (tool failed to run)
#   3 - Usage error

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Defaults & Config
# ─────────────────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

HEXVIEW_EXE="${HEXVIEW_EXE:-$PROJECT_ROOT/reference/HexView.exe}"
SCRATCHPAD="${SCRATCHPAD:-$PROJECT_ROOT/scratchpad}"

VERBOSE=0
KEEP_FILES=0
USE_SCRATCHPAD=0

# Colors (if terminal supports them)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    CYAN='\033[0;36m'
    NC='\033[0m' # No Color
else
    RED='' GREEN='' YELLOW='' CYAN='' NC=''
fi

# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

log() { echo -e "${CYAN}[compare]${NC} $*"; }
log_verbose() { [[ $VERBOSE -eq 1 ]] && echo -e "${CYAN}[compare]${NC} $*" || true; }
log_error() { echo -e "${RED}[error]${NC} $*" >&2; }
log_ok() { echo -e "${GREEN}[OK]${NC} $*"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $*"; }

usage() {
    sed -n '3,/^$/p' "$0" | sed 's/^# //' | sed 's/^#//'
    exit "${1:-0}"
}

# Convert WSL path to Windows path if needed
wsl_to_win_path() {
    local path="$1"
    if command -v wslpath &>/dev/null; then
        wslpath -w "$path"
    else
        echo "$path"
    fi
}

# Check if running in WSL
is_wsl() {
    [[ -f /proc/version ]] && grep -qi microsoft /proc/version
}

cleanup() {
    if [[ $KEEP_FILES -eq 0 && -n "${TMPDIR_CREATED:-}" ]]; then
        rm -rf "$TMPDIR_CREATED"
    fi
}
trap cleanup EXIT

# ─────────────────────────────────────────────────────────────────────────────
# Argument Parsing
# ─────────────────────────────────────────────────────────────────────────────

HEXVIEW_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        -v|--verbose) VERBOSE=1; shift ;;
        -k|--keep) KEEP_FILES=1; shift ;;
        -s|--scratchpad) USE_SCRATCHPAD=1; shift ;;
        -h|--help) usage 0 ;;
        --) shift; HEXVIEW_ARGS=("$@"); break ;;
        *) HEXVIEW_ARGS+=("$1"); shift ;;
    esac
done

if [[ ${#HEXVIEW_ARGS[@]} -eq 0 ]]; then
    log_error "No arguments provided"
    usage 3
fi

# ─────────────────────────────────────────────────────────────────────────────
# Find output file in args (look for -o or /O followed by filename)
# ─────────────────────────────────────────────────────────────────────────────

find_output_arg() {
    local args=("$@")
    local i=0
    while [[ $i -lt ${#args[@]} ]]; do
        local arg="${args[$i]}"
        # Handle -o filename or /O:filename patterns
        if [[ "$arg" =~ ^(-o|/[Oo])$ ]]; then
            # Next arg is the filename
            if [[ $((i + 1)) -lt ${#args[@]} ]]; then
                echo "${args[$((i + 1))]}"
                return 0
            fi
        elif [[ "$arg" =~ ^(-o|/[Oo])[=:](.+)$ ]]; then
            # Filename is part of this arg
            echo "${BASH_REMATCH[2]}"
            return 0
        fi
        ((i++))
    done
    return 1
}

OUTPUT_FILE=$(find_output_arg "${HEXVIEW_ARGS[@]}") || {
    log_error "Could not find output file in arguments (expected -o <file>)"
    exit 3
}

OUTPUT_BASE="${OUTPUT_FILE%.*}"
OUTPUT_EXT="${OUTPUT_FILE##*.}"
[[ "$OUTPUT_EXT" == "$OUTPUT_FILE" ]] && OUTPUT_EXT=""  # no extension

# ─────────────────────────────────────────────────────────────────────────────
# Set up working directory
# ─────────────────────────────────────────────────────────────────────────────

if [[ $USE_SCRATCHPAD -eq 1 ]]; then
    WORK_DIR="$SCRATCHPAD"
    mkdir -p "$WORK_DIR"
else
    WORK_DIR=$(mktemp -d)
    TMPDIR_CREATED="$WORK_DIR"
fi

# Output file paths
if [[ -n "$OUTPUT_EXT" ]]; then
    HEXVIEW_OUT="$WORK_DIR/${OUTPUT_BASE}_hexview.${OUTPUT_EXT}"
    CARGO_OUT="$WORK_DIR/${OUTPUT_BASE}_cargo.${OUTPUT_EXT}"
else
    HEXVIEW_OUT="$WORK_DIR/${OUTPUT_BASE}_hexview"
    CARGO_OUT="$WORK_DIR/${OUTPUT_BASE}_cargo"
fi

log_verbose "Work directory: $WORK_DIR"
log_verbose "HexView output: $HEXVIEW_OUT"
log_verbose "Cargo output:   $CARGO_OUT"

# ─────────────────────────────────────────────────────────────────────────────
# Build replacement args with new output paths
# ─────────────────────────────────────────────────────────────────────────────

replace_output_in_args() {
    local new_output="$1"
    shift
    local args=("$@")
    local result=()
    local i=0

    while [[ $i -lt ${#args[@]} ]]; do
        local arg="${args[$i]}"
        if [[ "$arg" =~ ^(-o|/[Oo])$ ]]; then
            result+=("$arg" "$new_output")
            ((i++))  # skip the original filename
        elif [[ "$arg" =~ ^(-o|/[Oo])[=:](.+)$ ]]; then
            result+=("-o" "$new_output")
        else
            result+=("$arg")
        fi
        ((i++))
    done

    printf '%s\n' "${result[@]}"
}

# ─────────────────────────────────────────────────────────────────────────────
# Run HexView.exe
# ─────────────────────────────────────────────────────────────────────────────

run_hexview() {
    log "Running HexView.exe..."

    if [[ ! -x "$HEXVIEW_EXE" && ! -f "$HEXVIEW_EXE" ]]; then
        log_error "HexView.exe not found at: $HEXVIEW_EXE"
        return 2
    fi

    # Build args with replaced output path
    local hexview_args
    mapfile -t hexview_args < <(replace_output_in_args "$HEXVIEW_OUT" "${HEXVIEW_ARGS[@]}")

    # For WSL, we may need to convert paths for input/output files
    if is_wsl && [[ $USE_SCRATCHPAD -eq 1 ]]; then
        # Convert output path to Windows format
        local win_out
        win_out=$(wsl_to_win_path "$HEXVIEW_OUT")
        mapfile -t hexview_args < <(replace_output_in_args "$win_out" "${HEXVIEW_ARGS[@]}")

        # Also convert input file if it's a path
        local first_arg="${hexview_args[0]}"
        if [[ -f "$first_arg" ]]; then
            hexview_args[0]=$(wsl_to_win_path "$first_arg")
        fi
    fi

    log_verbose "HexView args: ${hexview_args[*]}"

    if [[ $VERBOSE -eq 1 ]]; then
        "$HEXVIEW_EXE" "${hexview_args[@]}"
    else
        "$HEXVIEW_EXE" "${hexview_args[@]}" >/dev/null 2>&1
    fi
}

# ─────────────────────────────────────────────────────────────────────────────
# Run cargo (h3xy)
# ─────────────────────────────────────────────────────────────────────────────

run_cargo() {
    log "Running cargo run (h3xy)..."

    local cargo_args
    mapfile -t cargo_args < <(replace_output_in_args "$CARGO_OUT" "${HEXVIEW_ARGS[@]}")

    log_verbose "Cargo args: ${cargo_args[*]}"

    cd "$PROJECT_ROOT"

    if [[ $VERBOSE -eq 1 ]]; then
        cargo run --quiet -- "${cargo_args[@]}"
    else
        cargo run --quiet -- "${cargo_args[@]}" 2>&1
    fi
}

# ─────────────────────────────────────────────────────────────────────────────
# Compare outputs
# ─────────────────────────────────────────────────────────────────────────────

compare_outputs() {
    log "Comparing outputs..."

    if [[ ! -f "$HEXVIEW_OUT" ]]; then
        log_error "HexView output not found: $HEXVIEW_OUT"
        return 2
    fi

    if [[ ! -f "$CARGO_OUT" ]]; then
        log_error "Cargo output not found: $CARGO_OUT"
        return 2
    fi

    local hexview_size cargo_size
    hexview_size=$(wc -c < "$HEXVIEW_OUT")
    cargo_size=$(wc -c < "$CARGO_OUT")

    log_verbose "HexView output size: $hexview_size bytes"
    log_verbose "Cargo output size:   $cargo_size bytes"

    if cmp -s "$HEXVIEW_OUT" "$CARGO_OUT"; then
        return 0
    else
        return 1
    fi
}

show_diff() {
    if [[ $VERBOSE -eq 1 ]]; then
        echo ""
        log "Difference details:"

        # Check if files are text (hex files) or binary
        if file "$HEXVIEW_OUT" | grep -q text; then
            diff --color=auto -u "$HEXVIEW_OUT" "$CARGO_OUT" | head -50 || true
        else
            # Binary diff - show hex dump comparison
            echo "Binary files differ. First difference:"
            cmp -l "$HEXVIEW_OUT" "$CARGO_OUT" 2>/dev/null | head -20 || true
            echo ""
            echo "HexView (first 256 bytes):"
            xxd "$HEXVIEW_OUT" | head -16
            echo ""
            echo "Cargo (first 256 bytes):"
            xxd "$CARGO_OUT" | head -16
        fi
    fi
}

# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

main() {
    log "Comparing h3xy against HexView.exe"
    log_verbose "Arguments: ${HEXVIEW_ARGS[*]}"

    # Run HexView
    if ! run_hexview; then
        log_fail "HexView.exe execution failed"
        return 2
    fi

    # Run cargo
    if ! run_cargo; then
        log_fail "cargo run execution failed"
        return 2
    fi

    # Compare
    if compare_outputs; then
        log_ok "Outputs match! ✓"
        [[ $KEEP_FILES -eq 1 ]] && log "Files kept at: $HEXVIEW_OUT, $CARGO_OUT"
        return 0
    else
        log_fail "Outputs differ!"
        show_diff
        [[ $KEEP_FILES -eq 0 ]] && KEEP_FILES=1  # Keep files on failure for inspection
        log "Files preserved for inspection:"
        log "  HexView: $HEXVIEW_OUT"
        log "  Cargo:   $CARGO_OUT"
        return 1
    fi
}

main
