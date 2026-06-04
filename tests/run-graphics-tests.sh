#!/usr/bin/env bash
# Graphics golden-image regression tests for qbasic-rust.
#
# Each graphics program is run HEADLESS (no window) with a fixed RNG seed and a
# scripted key sequence via the runtime's QBC_* env-var driver, then its
# framebuffer checksum is compared against a committed golden in tests/golden/.
#
#   QBC_HEADLESS=1  — no window
#   QBC_SEED=N      — pin the RNG (overrides RANDOMIZE TIMER) for determinism
#   QBC_KEYS=...    — scripted keystrokes (one INKEY$ each)
#   QBC_EXIT_AFTER  — guaranteed termination (presents:N | ms:T | idle)
#   QBC_CHECKSUM=1  — print QBC_CHECKSUM=<hex> on exit
#
# Usage:
#   run-graphics-tests.sh                 # compare against goldens
#   run-graphics-tests.sh --write-golden  # (re)generate goldens
#   run-graphics-tests.sh -v              # verbose
#
# Exit code: 0 if all match, 1 otherwise.

set -euo pipefail

WRITE=0
VERBOSE=0
for arg in "$@"; do
    case "$arg" in
        --write-golden) WRITE=1 ;;
        -v|--verbose)   VERBOSE=1 ;;
        *) echo "Usage: run-graphics-tests.sh [--write-golden] [-v]" >&2; exit 1 ;;
    esac
done
vlog() { [ "$VERBOSE" -eq 1 ] && echo "$*" || true; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
QBC="$ROOT/target/release/qbc"
RLIB="$ROOT/target/release/libqbasic_runtime.rlib"
DEPS="$ROOT/target/release/deps"
SRC_DIR="$ROOT/basic-src"
GOLDEN_DIR="$SCRIPT_DIR/golden"
TMP="$SCRIPT_DIR/tmp-gfx"
mkdir -p "$GOLDEN_DIR" "$TMP"

# ── Test table: name | seed | keys | exit-policy ──────────────────────────────
# Deterministic programs first; add gorilla/donkey once their intros are pinned.
TESTS=(
    "reversi|1|Q|ms:8000"
    "mandel|1|ENTER|presents:80"
    "torus|1|ENTER|presents:5"
)

echo "Building runtime + transpiler (release)..."
cargo build --release --manifest-path "$ROOT/runtime/Cargo.toml" --quiet
cargo build --release --manifest-path "$ROOT/Cargo.toml" --quiet

pass=0; fail=0; wrote=0
errors=()

for entry in "${TESTS[@]}"; do
    IFS='|' read -r name seed keys exitp <<< "$entry"
    bas="$SRC_DIR/$name.bas"
    rs="$TMP/$name.rs"
    bin="$TMP/$name"
    golden="$GOLDEN_DIR/$name.txt"

    vlog ""
    vlog "── $name (seed=$seed keys=$keys exit=$exitp) ──"

    # Transpile + compile.
    "$QBC" "$bas" -o "$rs" >/dev/null 2>&1 || true
    if [ ! -f "$rs" ]; then
        echo "FAIL: $name  [transpile error]"; ((fail++)) || true
        errors+=("$name: transpile failed"); continue
    fi
    if ! rustc "$rs" --edition 2021 -L "$DEPS" \
            --extern qbasic_runtime="$RLIB" -o "$bin" 2>"$TMP/$name.cc"; then
        echo "FAIL: $name  [compile error]"
        grep "^error" "$TMP/$name.cc" | head -3 | sed 's/^/  /'
        ((fail++)) || true; errors+=("$name: compile failed"); continue
    fi

    # Run headless, capture checksum.
    out="$(QBC_HEADLESS=1 QBC_SEED="$seed" QBC_KEYS="$keys" \
           QBC_EXIT_AFTER="$exitp" QBC_CHECKSUM=1 \
           timeout 30 "$bin" 2>/dev/null || true)"
    sum="$(printf '%s\n' "$out" | grep -o 'QBC_CHECKSUM=[0-9a-f]*' | head -1 | cut -d= -f2)"
    if [ -z "$sum" ]; then
        echo "FAIL: $name  [no checksum — did it render/exit?]"
        ((fail++)) || true; errors+=("$name: no checksum"); continue
    fi

    if [ "$WRITE" -eq 1 ]; then
        echo "$sum" > "$golden"
        echo "WROTE: $name  ($sum)"
        ((wrote++)) || true
        continue
    fi

    if [ ! -f "$golden" ]; then
        echo "FAIL: $name  [no golden — run with --write-golden first]"
        ((fail++)) || true; errors+=("$name: missing golden"); continue
    fi

    want="$(cat "$golden")"
    if [ "$sum" = "$want" ]; then
        echo "PASS: $name  ($sum)"
        ((pass++)) || true
    else
        echo "FAIL: $name  [checksum mismatch]"
        echo "  expected $want"
        echo "  actual   $sum"
        # Dump the actual frame for visual inspection.
        QBC_HEADLESS=1 QBC_SEED="$seed" QBC_KEYS="$keys" QBC_EXIT_AFTER="$exitp" \
            QBC_DUMP="$GOLDEN_DIR/$name.actual.ppm" timeout 30 "$bin" >/dev/null 2>&1 || true
        echo "  wrote $GOLDEN_DIR/$name.actual.ppm for inspection"
        ((fail++)) || true; errors+=("$name: checksum mismatch")
    fi
done

rm -f "$TMP"/*.rs "$TMP"/*.cc 2>/dev/null || true
find "$TMP" -maxdepth 1 -type f -delete 2>/dev/null || true

echo ""
if [ "$WRITE" -eq 1 ]; then
    echo "Wrote $wrote golden(s) to $GOLDEN_DIR/"
    exit 0
fi
echo "Results: $pass passed, $fail failed"
if [ "$fail" -gt 0 ]; then
    echo ""
    echo "Failed:"
    for e in "${errors[@]}"; do echo "  - $e"; done
    exit 1
fi
