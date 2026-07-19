#!/usr/bin/env bash
# Differential fuzz driver: random QB programs (genfuzz.py) run through the
# qbc transpiler+rustc vs the independent reference interpreter (qbref.py).
# Any transpile failure, compile failure, crash, hang, or output mismatch is
# a finding; failing cases are saved to tools/fuzz/failures/.
#
# Usage: bash tools/fuzz/run-fuzz.sh [count] [start-seed]
set -u

COUNT="${1:-100}"
START="${2:-1}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/../.."
QBC="$ROOT/target/release/qbc"
RLIB="$ROOT/target/release/libqbasic_runtime.rlib"
DEPS="$ROOT/target/release/deps"
TMP="$SCRIPT_DIR/tmp"
FAIL_DIR="$SCRIPT_DIR/failures"
mkdir -p "$TMP" "$FAIL_DIR"

# Refresh the runtime rlib FIRST — a root `cargo build` does not update the
# non-hashed rlib the manual rustc link uses (the documented stale-rlib trap).
(cd "$ROOT" && cargo build --release -p qbasic_runtime -q && cargo build --release -q)

# Portable 10s guard (timeout(1) is not installed on macOS by default).
with_timeout() { perl -e 'alarm shift; exec @ARGV' 10 "$@"; }

pass=0; fail=0
for ((seed = START; seed < START + COUNT; seed++)); do
    base="$TMP/fz$seed"
    python3 "$SCRIPT_DIR/genfuzz.py" "$seed" > "$base.bas"

    if ! "$QBC" "$base.bas" --emit-only -o "$base.rs" > /dev/null 2> "$base.err"; then
        echo "FAIL seed=$seed  [transpile error]"
        cp "$base.bas" "$FAIL_DIR/"; cp "$base.err" "$FAIL_DIR/fz$seed.err"
        ((fail++)); continue
    fi
    if ! rustc --edition 2021 -O "$base.rs" \
            --extern qbasic_runtime="$RLIB" -L "$DEPS" \
            -o "$base.bin" 2> "$base.err"; then
        echo "FAIL seed=$seed  [rustc error]"
        cp "$base.bas" "$FAIL_DIR/"; cp "$base.err" "$FAIL_DIR/fz$seed.err"
        ((fail++)); continue
    fi
    if ! with_timeout env QBC_HEADLESS=1 "$base.bin" > "$base.qbc.out" 2>/dev/null; then
        echo "FAIL seed=$seed  [runtime crash or hang]"
        cp "$base.bas" "$FAIL_DIR/"
        ((fail++)); continue
    fi
    if ! with_timeout python3 "$SCRIPT_DIR/qbref.py" "$base.bas" > "$base.ref.out" 2> "$base.err"; then
        echo "FAIL seed=$seed  [oracle error — qbref bug or subset drift]"
        cp "$base.bas" "$FAIL_DIR/"; cp "$base.err" "$FAIL_DIR/fz$seed.err"
        ((fail++)); continue
    fi
    if ! diff -q "$base.qbc.out" "$base.ref.out" > /dev/null; then
        echo "FAIL seed=$seed  [OUTPUT MISMATCH]"
        cp "$base.bas" "$FAIL_DIR/"
        cp "$base.qbc.out" "$FAIL_DIR/fz$seed.qbc.out"
        cp "$base.ref.out" "$FAIL_DIR/fz$seed.ref.out"
        ((fail++)); continue
    fi
    ((pass++))
done

echo
echo "Fuzz results: $pass passed, $fail failed  (seeds $START..$((START + COUNT - 1)))"
[ "$fail" -eq 0 ]
