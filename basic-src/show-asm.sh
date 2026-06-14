#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
QBC="$ROOT/target/release/qbc"
BIN_DIR="$ROOT/bin"

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <file.bas>" >&2
    exit 1
fi

BAS="$1"
if [[ ! -f "$BAS" ]]; then
    echo "Error: '$BAS' not found" >&2
    exit 1
fi

NAME="$(basename "$BAS" .bas)"
RS="$BIN_DIR/$NAME.rs"
ASM="$BIN_DIR/$NAME.asm"

mkdir -p "$BIN_DIR"

# Build qbc and runtime if needed
cargo build --manifest-path "$ROOT/runtime/Cargo.toml" --release --quiet
cargo build --manifest-path "$ROOT/Cargo.toml" --bin qbc --release --quiet

# Transpile .bas → .rs
echo "Transpiling $BAS → $RS"
"$QBC" "$BAS" -o "$RS" --emit-only

# Compile .rs → binary + .asm (Intel syntax, optimized)
# --out-dir puts both the binary and the .s in bin/
echo "Compiling $RS → $ASM"
rustc "$RS" \
    --edition 2021 \
    -L "$ROOT/target/release/deps" \
    --extern qbasic_runtime="$ROOT/target/release/libqbasic_runtime.rlib" \
    -C opt-level=3 \
    -C llvm-args='-x86-asm-syntax=intel' \
    --emit=asm,link \
    --out-dir "$BIN_DIR"

# rustc names it <crate>.s; rename to .asm
if [[ -f "$BIN_DIR/$NAME.s" ]]; then
    mv "$BIN_DIR/$NAME.s" "$ASM"
fi

echo "Done:"
echo "  Source:   $RS"
echo "  Assembly: $ASM"
echo "  Binary:   $BIN_DIR/$NAME"
