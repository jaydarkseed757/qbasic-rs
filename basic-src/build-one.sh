#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <file.bas>"
    exit 1
fi

BAS="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
BIN_DIR="$ROOT/bin"
QBC="$ROOT/target/release/qbc"

name="$(basename "$BAS" .bas)"
RS="$BIN_DIR/$name.rs"
BIN="$BIN_DIR/$name"

mkdir -p "$BIN_DIR"

# Build runtime and qbc (release) so everything is up to date
cargo build --manifest-path "$ROOT/runtime/Cargo.toml" --release --quiet
cargo build --manifest-path "$ROOT/Cargo.toml" --bin qbc --release --quiet

echo "==> Transpiling $BAS"
"$QBC" "$BAS" -o "$RS" --verbose

echo ""
echo "==> Compiling $RS"
rustc "$RS" --edition 2021 \
    -L "$ROOT/target/release/deps" \
    --extern qbasic_runtime="$ROOT/target/release/libqbasic_runtime.rlib" \
    -o "$BIN"

echo ""
echo "==> Built: $BIN"
