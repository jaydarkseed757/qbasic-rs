#!/usr/bin/env bash
# Test runner for qbasic-rust regression tests.
#
# For each tests/programs/*.bas:
#   1. Transpile with qbc
#   2. Compile with rustc
#   3. Run binary, capture stdout
#   4. Diff against tests/expected/<name>.txt
#   5. Report PASS / FAIL
#
# Exit code: 0 if all pass, 1 if any fail.

set -euo pipefail

# ── Options ───────────────────────────────────────────────────────────────────
VERBOSE=0
KEEP_RS=0
for arg in "$@"; do
    case "$arg" in
        -v|--verbose) VERBOSE=1 ;;
        -d|--debug)   KEEP_RS=1 ;;
        *) echo "Usage: run-tests.sh [-v] [-d]" >&2; exit 1 ;;
    esac
done

vlog() { [ "$VERBOSE" -eq 1 ] && echo "$*" || true; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
QBC="$ROOT/target/release/qbc"
RLIB="$ROOT/target/release/libqbasic_runtime.rlib"
DEPS="$ROOT/target/release/deps"
TMP="$SCRIPT_DIR/tmp"
mkdir -p "$TMP"
PROG_DIR="$SCRIPT_DIR/programs"
EXPECTED_DIR="$SCRIPT_DIR/expected"

pass=0
fail=0
errors=()

# Always rebuild runtime and transpiler so edits to runtime/src/lib.rs or the
# transpiler are picked up. Cargo is incremental, so this is cheap when nothing
# changed. Note: a root `cargo build` does NOT refresh the non-hashed
# target/release/libqbasic_runtime.rlib that the test loop links against — only
# building via the runtime's own manifest updates that path. Gating on file
# existence (the old behaviour) silently linked a stale rlib after runtime edits.
echo "Building runtime..."
cargo build --release --manifest-path "$ROOT/runtime/Cargo.toml" --quiet

echo "Building transpiler..."
cargo build --release --manifest-path "$ROOT/Cargo.toml" --quiet

for bas in "$PROG_DIR"/*.bas; do
    name="$(basename "$bas" .bas)"
    expected="$EXPECTED_DIR/$name.txt"
    rs="$TMP/$name.rs"
    bin="$TMP/$name"
    actual="$TMP/$name.out"

    vlog ""
    vlog "── $name ──────────────────────────────"

    # Skip if no expected file
    if [ ! -f "$expected" ]; then
        echo "SKIP: $name (no expected file)"
        continue
    fi

    # 1. Transpile
    transpile_out="$("$QBC" "$bas" -o "$rs" 2>&1 || true)"
    if [ ! -f "$rs" ]; then
        echo "FAIL: $name  [transpile error]"
        vlog "  transpile output:"
        vlog "  $transpile_out"
        ((fail++)) || true
        errors+=("$name: transpile failed")
        continue
    fi
    vlog "  transpile: ok  ($rs)"

    # 2. Compile
    compile_out="$(rustc "$rs" --edition 2021 \
        -L "$DEPS" \
        --extern qbasic_runtime="$RLIB" \
        -o "$bin" 2>&1 || true)"
    if [ ! -x "$bin" ]; then
        echo "FAIL: $name  [compile error]"
        echo "$compile_out" | grep "^error" | head -3 | sed 's/^/  /'
        vlog "  full compile output:"
        vlog "$compile_out" | sed 's/^/    /'
        ((fail++)) || true
        errors+=("$name: compile failed")
        continue
    fi
    vlog "  compile:   ok  ($bin)"

    # 3. Run (5s timeout)
    if ! timeout 5 "$bin" > "$actual" 2>/dev/null; then
        echo "FAIL: $name  [runtime error or timeout]"
        ((fail++)) || true
        errors+=("$name: runtime error/timeout")
        continue
    fi
    vlog "  run:       ok  ($actual)"

    # 4. Diff
    if diff -q "$expected" "$actual" > /dev/null 2>&1; then
        echo "PASS: $name"
        if [ "$VERBOSE" -eq 1 ]; then
            echo "  output:"
            sed 's/^/    /' "$actual"
        fi
        ((pass++)) || true
    else
        echo "FAIL: $name  [output mismatch]"
        diff --label expected --label actual "$expected" "$actual" | head -30 | sed 's/^/  /'
        ((fail++)) || true
        errors+=("$name: output mismatch")
    fi
done

if [ "$KEEP_RS" -eq 1 ]; then
    # Remove binaries and .out files but keep .rs files
    find "$TMP" -maxdepth 1 -type f ! -name '*.rs' -delete
    echo ""
    echo "Generated .rs files kept in: $TMP/"
    ls "$TMP"/*.rs 2>/dev/null | sed "s|$TMP/|  |"
else
    rm -f "$TMP"/*.rs "$TMP"/*.out
    find "$TMP" -maxdepth 1 -type f -delete
fi

echo ""
echo "Results: $pass passed, $fail failed"

if [ $fail -gt 0 ]; then
    echo ""
    echo "Failed tests:"
    for e in "${errors[@]}"; do
        echo "  - $e"
    done
    exit 1
fi
