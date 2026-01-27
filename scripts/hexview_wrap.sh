#!/usr/bin/env bash
#
# hexview_wrap.sh - tiny wrapper to run HexView.exe from WSL
#
# Usage:
#   ./scripts/hexview_wrap.sh [OPTIONS] -- <hexview_args>
#
# Options:
#   -s, --scratchpad  Copy input/output to scratchpad (Windows-visible)
#   -w, --wslpath     Convert input/output paths via wslpath -w
#   -x, --windows     Windows-mode: -s -w + run via cmd.exe
#   -n, --dry-run     Print command, do not execute
#   -h, --help        Show help
#
# Environment:
#   HEXVIEW_EXE       Path to HexView.exe (default: ./reference/HexView.exe)
#   SCRATCHPAD        Path to scratchpad directory (default: ./scratchpad)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

HEXVIEW_EXE="${HEXVIEW_EXE:-$PROJECT_ROOT/reference/HexView.exe}"
SCRATCHPAD="${SCRATCHPAD:-$PROJECT_ROOT/scratchpad}"

USE_SCRATCHPAD=0
USE_WSLPATH=0
USE_CMD=0
DRY_RUN=0

usage() {
    sed -n '3,/^$/p' "$0" | sed 's/^# //' | sed 's/^#//'
    exit "${1:-0}"
}

wsl_to_win_path() {
    local path="$1"
    if command -v wslpath &>/dev/null; then
        wslpath -w "$path"
    else
        echo "$path"
    fi
}

resolve_path() {
    local path="$1"
    if command -v realpath &>/dev/null; then
        realpath "$path"
    else
        readlink -f "$path" 2>/dev/null || echo "$path"
    fi
}

find_output_arg() {
    local args=("$@")
    local i=0
    while [[ $i -lt ${#args[@]} ]]; do
        local arg="${args[$i]}"
        if [[ "$arg" =~ ^(-o|/[Oo])$ ]]; then
            if [[ $((i + 1)) -lt ${#args[@]} ]]; then
                echo "${args[$((i + 1))]}"
                return 0
            fi
        elif [[ "$arg" =~ ^(-o|/[Oo])[=:](.+)$ ]]; then
            echo "${BASH_REMATCH[2]}"
            return 0
        elif [[ "$arg" =~ ^-o(.+)$ && "$arg" != "-offset" ]]; then
            echo "${BASH_REMATCH[1]}"
            return 0
        elif [[ "$arg" =~ ^/[Oo](.+)$ ]]; then
            echo "${BASH_REMATCH[1]}"
            return 0
        fi
        i=$((i + 1))
    done
    return 1
}

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
            i=$((i + 1))
        elif [[ "$arg" =~ ^(-o|/[Oo])[=:](.+)$ ]]; then
            result+=("-o" "$new_output")
        elif [[ "$arg" =~ ^-o(.+)$ && "$arg" != "-offset" ]]; then
            result+=("-o$new_output")
        elif [[ "$arg" =~ ^/[Oo](.+)$ ]]; then
            result+=("/O$new_output")
        else
            result+=("$arg")
        fi
        i=$((i + 1))
    done

    printf '%s\n' "${result[@]}"
}

find_input_arg() {
    local args=("$@")
    local i=0
    while [[ $i -lt ${#args[@]} ]]; do
        local arg="${args[$i]}"
        if [[ -f "$arg" ]]; then
            echo "${i}:${arg}"
            return 0
        fi
        i=$((i + 1))
    done
    return 1
}

ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        -s|--scratchpad) USE_SCRATCHPAD=1; shift ;;
        -w|--wslpath) USE_WSLPATH=1; shift ;;
        -x|--windows) USE_SCRATCHPAD=1; USE_WSLPATH=1; USE_CMD=1; shift ;;
        -n|--dry-run) DRY_RUN=1; shift ;;
        -h|--help) usage 0 ;;
        --) shift; ARGS=("$@"); break ;;
        *) ARGS+=("$1"); shift ;;
    esac
done

if [[ ${#ARGS[@]} -eq 0 ]]; then
    usage 1
fi

if [[ ! -x "$HEXVIEW_EXE" && ! -f "$HEXVIEW_EXE" ]]; then
    echo "[wrap] HexView.exe not found: $HEXVIEW_EXE" >&2
    exit 2
fi

if [[ $USE_SCRATCHPAD -eq 1 ]]; then
    mkdir -p "$SCRATCHPAD"

    input_info="$(find_input_arg "${ARGS[@]}" || true)"
    if [[ -n "$input_info" ]]; then
        input_idx="${input_info%%:*}"
        input_path="${input_info#*:}"
        input_copy="$SCRATCHPAD/$(basename "$input_path")"
        cp -f "$input_path" "$input_copy"
        ARGS[$input_idx]="$input_copy"
    fi

    output_path="$(find_output_arg "${ARGS[@]}" || true)"
    if [[ -n "$output_path" ]]; then
        output_copy="$SCRATCHPAD/$(basename "$output_path")"
        mapfile -t ARGS < <(replace_output_in_args "$output_copy" "${ARGS[@]}")
    fi
fi

if [[ $USE_WSLPATH -eq 1 ]]; then
    output_path="$(find_output_arg "${ARGS[@]}" || true)"
    if [[ -n "$output_path" ]]; then
        resolved_out="$(resolve_path "$output_path")"
        win_out="$(wsl_to_win_path "$resolved_out")"
        mapfile -t ARGS < <(replace_output_in_args "$win_out" "${ARGS[@]}")
    fi

    input_info="$(find_input_arg "${ARGS[@]}" || true)"
    if [[ -n "$input_info" ]]; then
        input_idx="${input_info%%:*}"
        input_path="${input_info#*:}"
        resolved_in="$(resolve_path "$input_path")"
        ARGS[$input_idx]="$(wsl_to_win_path "$resolved_in")"
    fi
fi

echo "[wrap] HEXVIEW_EXE: $HEXVIEW_EXE"
echo "[wrap] ARGS: ${ARGS[*]}"

if [[ $DRY_RUN -eq 1 ]]; then
    exit 0
fi

if [[ $USE_CMD -eq 1 ]]; then
    if ! command -v cmd.exe &>/dev/null; then
        echo "[wrap] cmd.exe not found in PATH" >&2
        exit 2
    fi

    win_exe="$(wsl_to_win_path "$HEXVIEW_EXE")"
    cmdline="\"$win_exe\""
    for arg in "${ARGS[@]}"; do
        if [[ "$arg" =~ [[:space:]] ]]; then
            cmdline+=" \"${arg}\""
        else
            cmdline+=" ${arg}"
        fi
    done

    echo "[wrap] CMDLINE: $cmdline"
    cmd.exe /c "$cmdline"
else
    "$HEXVIEW_EXE" "${ARGS[@]}"
fi
