#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
QBC="$ROOT/target/release/qbc"

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <file.bas>" >&2
    echo "Prints qbc --compatibility's QBasic 1.1/QuickBASIC 4.5/GW-BASIC dialect-fidelity report for <file.bas>." >&2
    exit 1
fi

BAS="$1"
if [[ ! -f "$BAS" ]]; then
    echo "Error: '$BAS' not found" >&2
    exit 1
fi

# Build qbc if needed (--compatibility is a standalone analysis mode — it
# exits right after parsing, never invokes rustc, so only qbc itself needs
# building, not the runtime crate).
cargo build --manifest-path "$ROOT/Cargo.toml" --bin qbc --release --quiet

"$QBC" "$BAS" --compatibility
