#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
QBC="$ROOT/target/release/qbc"
BIN_DIR="$ROOT/bin"

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <file.bas>" >&2
    echo "Prints qbc --explain's GameState field-origin report for <file.bas>." >&2
    exit 1
fi

BAS="$1"
if [[ ! -f "$BAS" ]]; then
    echo "Error: '$BAS' not found" >&2
    exit 1
fi

NAME="$(basename "$BAS" .bas)"
RS="$BIN_DIR/$NAME.rs"

mkdir -p "$BIN_DIR"

# Build qbc if needed (the runtime crate isn't touched by --explain, but keep
# the same up-to-date-build guarantee show-asm.sh gives).
cargo build --manifest-path "$ROOT/Cargo.toml" --bin qbc --release --quiet

# --emit-only: we just want the report, not a compiled binary. The .rs is
# still written to bin/ (and is BYTE-IDENTICAL to a plain, non---explain
# transpile — --explain never changes the emitted Rust) so it's there if
# you want to cross-reference the report against the actual GameState struct.
"$QBC" "$BAS" -o "$RS" --emit-only --explain
