#!/usr/bin/env bash
# Graphics DETERMINISM fuzz driver: random SCREEN 13 drawing programs
# (genfuzz_gfx.py) are transpiled + compiled ONCE, then run headless TWICE
# with identical env. The two QBC_CHECKSUM values must match bit-for-bit —
# there's no independent renderer to diff against (unlike run-fuzz.sh's
# text-mode oracle), so this checks the property that actually broke before
# the simulated headless clock: SAME program, SAME seed, SAME checksum,
# every time, regardless of wall-clock timing. A crash or hang counts as a
# finding too. Failures are saved to tools/fuzz/gfx-failures/.
#
# Usage: bash tools/fuzz/run-fuzz-gfx.sh [count] [start-seed]
set -u

COUNT="${1:-50}"
START="${2:-1}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/../.."
QBC="$ROOT/target/release/qbc"
RLIB="$ROOT/target/release/libqbasic_runtime.rlib"
DEPS="$ROOT/target/release/deps"
TMP="$SCRIPT_DIR/tmp-gfx"
FAIL_DIR="$SCRIPT_DIR/gfx-failures"
mkdir -p "$TMP" "$FAIL_DIR"

(cd "$ROOT" && cargo build --release -p qbasic_runtime -q && cargo build --release -q)

# Portable 10s guard (timeout(1) is not installed on macOS by default).
with_timeout() { perl -e 'alarm shift; exec @ARGV' 10 "$@"; }

checksum_of() {
    with_timeout env QBC_HEADLESS=1 QBC_SEED=42 QBC_CHECKSUM=1 "$1" 2>/dev/null \
        | grep -o 'QBC_CHECKSUM=[0-9a-f]*' | head -1
}

pass=0; fail=0
for ((seed = START; seed < START + COUNT; seed++)); do
    base="$TMP/gfx$seed"
    python3 "$SCRIPT_DIR/genfuzz_gfx.py" "$seed" > "$base.bas"

    if ! "$QBC" "$base.bas" --emit-only -o "$base.rs" > /dev/null 2> "$base.err"; then
        echo "FAIL seed=$seed  [transpile error]"
        cp "$base.bas" "$FAIL_DIR/"; cp "$base.err" "$FAIL_DIR/gfx$seed.err"
        ((fail++)); continue
    fi
    # -O is safe here (unlike golden checksums, nothing is compared against a
    # STORED value across builds — both runs below use this one binary).
    if ! rustc --edition 2021 -O "$base.rs" \
            --extern qbasic_runtime="$RLIB" -L "$DEPS" \
            -o "$base.bin" 2> "$base.err"; then
        echo "FAIL seed=$seed  [rustc error]"
        cp "$base.bas" "$FAIL_DIR/"; cp "$base.err" "$FAIL_DIR/gfx$seed.err"
        ((fail++)); continue
    fi

    sum1="$(checksum_of "$base.bin")"
    if [ -z "$sum1" ]; then
        echo "FAIL seed=$seed  [run 1: crash or hang — no checksum]"
        cp "$base.bas" "$FAIL_DIR/"
        ((fail++)); continue
    fi
    sum2="$(checksum_of "$base.bin")"
    if [ -z "$sum2" ]; then
        echo "FAIL seed=$seed  [run 2: crash or hang — no checksum]"
        cp "$base.bas" "$FAIL_DIR/"
        ((fail++)); continue
    fi
    if [ "$sum1" != "$sum2" ]; then
        echo "FAIL seed=$seed  [NONDETERMINISTIC: $sum1 vs $sum2]"
        cp "$base.bas" "$FAIL_DIR/"
        ((fail++)); continue
    fi
    ((pass++))
done

echo
echo "Graphics determinism fuzz: $pass passed, $fail failed  (seeds $START..$((START + COUNT - 1)))"
[ "$fail" -eq 0 ]
