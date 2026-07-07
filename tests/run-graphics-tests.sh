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
# Only programs whose snapshot is DETERMINISTIC belong here. A good golden either
# (a) draws once and stops, or (b) finishes its draw well within the exit window
# so the snapshot always lands on the completed image.
#
# Deliberately EXCLUDED:
#   mandel  — draws slowly, then palette-cycles forever. Any snapshot is either
#             mid-render (timing-dependent under load) or post-render with a
#             mutated palette, so its checksum is not reproducible. Verified
#             headless by hand instead (QBC_FBSTATS) — see notes in run output.
#   gorilla/donkey — more input + animation; add once their intros are scripted.
TESTS=(
    # SCREEN 13 palette demos — no input, no RND, terminate on their own
    # (bare SLEEP is a no-op → Drop dumps); the ms cap is just a safety net.
    "256c|1||ms:3000"
    "screen13|1||ms:3000"
    "screen13-sprite|1||ms:3000"
    "palette256_expanded|1||ms:3000"
    # reversi: static board, no palette cycling — fully deterministic.
    "reversi|1|Q|ms:8000"
    # torus: the render completes within the first few frame-intervals, so
    # presents:5 always captures the finished torus before palette rotation.
    "torus|1|ENTER|presents:5"
    # hangman-gfx: SCREEN 12, word chosen by RND (pinned via seed), gallows
    # drawn before the first INPUT prompt — presents:1 captures the initial
    # board (empty gallows + underscores) without needing any key input.
    "hangman-gfx|1||presents:1"
    # duck: SCREEN 9, pure DRAW+PAINT scene, fully deterministic (no RND).
    "duck|1||presents:1"
    # gorilla: scripted through intro+inputs+GorillaIntro, then one throw
    # (angle 45°, velocity 50); captures the banana mid-flight.  DRAIN tokens
    # stop the WHILE INKEY$<>"":WEND drain-loops in SparklePause and GetNum#.
    # presents:80 always lands on a mid-flight frame with seed 42.
    "gorilla|42|DRAIN,ENTER,ENTER,ENTER,1,ENTER,9.8,ENTER,P,DRAIN,4,5,ENTER,DRAIN,5,0,ENTER|presents:80"
    # donkey: SCREEN 1 (CGA), DRAIN stops title-screen buffer flush, SPACE
    # continues past the title.  presents:10 captures road + car + donkey all
    # visible in their seed-42 positions.
    "donkey|42|DRAIN,SPACE|presents:10"
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
    # `|| true` guards the whole pipe: under `set -o pipefail`, a run that
    # produced no checksum (crashed/timed out) makes `grep -o` return 1 (no
    # match), which — without this guard — aborts the ENTIRE script right
    # here via `set -e`, silently skipping every remaining test and the
    # final "Results:" summary instead of reporting "no checksum" for just
    # this one test and continuing.
    sum="$(printf '%s\n' "$out" | grep -o 'QBC_CHECKSUM=[0-9a-f]*' | head -1 | cut -d= -f2 || true)"
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
