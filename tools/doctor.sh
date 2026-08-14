#!/usr/bin/env bash
# qbc doctor — single entrypoint for full project health.
#
# Runs, in order: unit tests, integration tests, the full basic-src build
# (incremental by default), and the graphics golden-checksum tests. Every
# stage always runs, even if an earlier one failed, so one invocation
# surfaces every problem instead of just the first.
#
# Usage:
#   doctor.sh            # incremental build-all, terse sub-script output
#   doctor.sh --full     # force a from-scratch build-all (no cache reuse)
#   doctor.sh -v         # show full sub-script output, not just the tail
#
# Exit code: 0 if every stage passed, 1 otherwise.

FULL=0
VERBOSE=0
for arg in "$@"; do
    case "$arg" in
        --full)       FULL=1 ;;
        -v|--verbose) VERBOSE=1 ;;
        *) echo "Usage: doctor.sh [--full] [-v]" >&2; exit 1 ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT

names=()
results=()
times=()
notes=()

run_stage() {
    local label="$1"; shift
    local start end dur status note=""

    printf '\n=== %s ===\n' "$label"
    start="$(date +%s)"
    if [ "$VERBOSE" -eq 1 ]; then
        "$@" 2>&1 | tee "$LOG"
        status="${PIPESTATUS[0]}"
    else
        "$@" >"$LOG" 2>&1
        status=$?
        tail -n 15 "$LOG"
    fi
    end="$(date +%s)"
    dur=$((end - start))

    if [ "$label" = "build-all (bundled)" ]; then
        note="$(grep -o '[0-9]* passed ([0-9]* cached)' "$LOG" | tail -1 || true)"
        note="${note:+  ($note)}"
    fi

    names+=("$label")
    times+=("${dur}s")
    notes+=("$note")
    if [ "$status" -eq 0 ]; then
        results+=("PASS")
    else
        results+=("FAIL")
    fi
}

total_start="$(date +%s)"

run_stage "Unit tests" \
    cargo test --workspace --manifest-path "$ROOT/Cargo.toml" --quiet

run_stage "Integration tests" \
    bash "$ROOT/tests/run-tests.sh" $([ "$VERBOSE" -eq 1 ] && echo -v)

if [ "$FULL" -eq 1 ]; then
    run_stage "build-all (bundled)" bash "$ROOT/basic-src/build-all.sh" --clean
else
    run_stage "build-all (bundled)" bash "$ROOT/basic-src/build-all.sh"
fi

run_stage "Graphics goldens" \
    bash "$ROOT/tests/run-graphics-tests.sh" $([ "$VERBOSE" -eq 1 ] && echo -v)

total_end="$(date +%s)"
total_dur=$((total_end - total_start))

# ── Summary ─────────────────────────────────────────────────────────────────
echo ""
printf '  %-25s %-6s %6s\n' "Stage" "Result" "Time"
printf '  %s\n' "────────────────────────────────────────"
fail_count=0
failed_stages=()
for i in "${!names[@]}"; do
    printf '  %-25s %-6s %6s%s\n' "${names[$i]}" "${results[$i]}" "${times[$i]}" "${notes[$i]}"
    if [ "${results[$i]}" != "PASS" ]; then
        fail_count=$((fail_count + 1))
        failed_stages+=("${names[$i]}")
    fi
done
printf '  %s\n' "────────────────────────────────────────"

if [ "$fail_count" -eq 0 ]; then
    echo "  qbc doctor: ALL CLEAR  (${total_dur}s total)"
    exit 0
else
    echo "  qbc doctor: $fail_count stage(s) FAILED  (${total_dur}s total)"
    echo "  Failed: ${failed_stages[*]}"
    exit 1
fi
