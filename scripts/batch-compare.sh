#!/usr/bin/env bash
#
# batch-compare.sh - Run multiple comparison tests against HexView.exe
#
# Usage:
#   ./scripts/batch-compare.sh [OPTIONS] [test_file]
#
# If test_file is provided, run tests from that file.
# Otherwise, run built-in smoke tests.
#
# Test file format (one test per line):
#   # Comments start with hash
#   TEST_NAME: arg1 arg2 -o output.hex
#
# Options:
#   -v, --verbose     Show detailed output for each test
#   -s, --stop        Stop on first failure
#   -l, --list        List available tests without running
#   -h, --help        Show this help message

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPARE_SCRIPT="$SCRIPT_DIR/compare.sh"

# Colors
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' CYAN='' BOLD='' NC=''
fi

VERBOSE=0
STOP_ON_FAIL=0
LIST_ONLY=0
TEST_FILE=""

# ─────────────────────────────────────────────────────────────────────────────
# Built-in smoke tests (used when no test file provided)
# ─────────────────────────────────────────────────────────────────────────────

# These assume a test input file exists. Adjust paths as needed.
declare -A BUILTIN_TESTS=(
    ["passthrough"]="test.hex -o out.hex"
    ["address_range"]="test.hex /AR:'0x0000-0x00FF' -o out.hex"
    ["fill_range"]="test.hex /FR:'0x1000,0x100' /FP:DEADBEEF -o out.hex"
    ["cut_range"]="test.hex /CR:'0x0080-0x00FF' -o out.hex"
    ["fill_all"]="test.hex /FA /FP:00 -o out.hex"
    ["align_data"]="test.hex /AD:4 -o out.hex"
    ["split_blocks"]="test.hex /SB:0x80 -o out.hex"
    ["swap_word"]="test.hex /SWAPWORD -o out.hex"
    ["swap_long"]="test.hex /SWAPLONG -o out.hex"
)

# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

log() { echo -e "${CYAN}[batch]${NC} $*"; }
log_error() { echo -e "${RED}[error]${NC} $*" >&2; }

usage() {
    sed -n '3,/^$/p' "$0" | sed 's/^# //' | sed 's/^#//'
    exit "${1:-0}"
}

# ─────────────────────────────────────────────────────────────────────────────
# Argument Parsing
# ─────────────────────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        -v|--verbose) VERBOSE=1; shift ;;
        -s|--stop) STOP_ON_FAIL=1; shift ;;
        -l|--list) LIST_ONLY=1; shift ;;
        -h|--help) usage 0 ;;
        *) TEST_FILE="$1"; shift ;;
    esac
done

# ─────────────────────────────────────────────────────────────────────────────
# Load tests
# ─────────────────────────────────────────────────────────────────────────────

declare -A TESTS

load_tests_from_file() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        log_error "Test file not found: $file"
        exit 3
    fi

    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip empty lines and comments
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue

        # Parse "NAME: args" format
        if [[ "$line" =~ ^([^:]+):[[:space:]]*(.+)$ ]]; then
            local name="${BASH_REMATCH[1]}"
            local args="${BASH_REMATCH[2]}"
            TESTS["$name"]="$args"
        fi
    done < "$file"
}

load_builtin_tests() {
    for name in "${!BUILTIN_TESTS[@]}"; do
        TESTS["$name"]="${BUILTIN_TESTS[$name]}"
    done
}

if [[ -n "$TEST_FILE" ]]; then
    load_tests_from_file "$TEST_FILE"
else
    load_builtin_tests
fi

# ─────────────────────────────────────────────────────────────────────────────
# List mode
# ─────────────────────────────────────────────────────────────────────────────

if [[ $LIST_ONLY -eq 1 ]]; then
    echo "Available tests:"
    for name in $(echo "${!TESTS[@]}" | tr ' ' '\n' | sort); do
        echo "  ${BOLD}$name${NC}: ${TESTS[$name]}"
    done
    exit 0
fi

# ─────────────────────────────────────────────────────────────────────────────
# Run tests
# ─────────────────────────────────────────────────────────────────────────────

PASSED=0
FAILED=0
SKIPPED=0
FAILED_TESTS=()

run_test() {
    local name="$1"
    local args="$2"

    printf "  %-30s " "$name"

    local compare_opts=""
    [[ $VERBOSE -eq 1 ]] && compare_opts="-v"

    # Parse args string into array (respecting quotes)
    local cmd_args
    eval "cmd_args=($args)"

    if "$COMPARE_SCRIPT" $compare_opts -- "${cmd_args[@]}" 2>&1; then
        echo -e "${GREEN}PASS${NC}"
        ((PASSED++))
        return 0
    else
        local rc=$?
        if [[ $rc -eq 2 ]]; then
            echo -e "${YELLOW}SKIP${NC} (execution error)"
            ((SKIPPED++))
        else
            echo -e "${RED}FAIL${NC}"
            ((FAILED++))
            FAILED_TESTS+=("$name")
        fi
        return $rc
    fi
}

# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  h3xy vs HexView.exe Comparison Tests${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

log "Running ${#TESTS[@]} tests..."
echo ""

for name in $(echo "${!TESTS[@]}" | tr ' ' '\n' | sort); do
    if ! run_test "$name" "${TESTS[$name]}"; then
        if [[ $STOP_ON_FAIL -eq 1 ]]; then
            echo ""
            log_error "Stopping on first failure"
            break
        fi
    fi
done

# ─────────────────────────────────────────────────────────────────────────────
# Summary
# ─────────────────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}───────────────────────────────────────────────────────────────${NC}"
echo -e "Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC}, ${YELLOW}$SKIPPED skipped${NC}"

if [[ ${#FAILED_TESTS[@]} -gt 0 ]]; then
    echo ""
    echo "Failed tests:"
    for t in "${FAILED_TESTS[@]}"; do
        echo "  - $t"
    done
fi

echo -e "${BOLD}───────────────────────────────────────────────────────────────${NC}"
echo ""

[[ $FAILED -eq 0 ]]
