#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
QBC="$ROOT/target/release/qbc"

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <file.bas>" >&2
    echo "Prints qbc --opt-report's source-level findings report for <file.bas>." >&2
    exit 1
fi

BAS="$1"
if [[ ! -f "$BAS" ]]; then
    echo "Error: '$BAS' not found" >&2
    exit 1
fi

# --opt-report runs after analyze() but is still a standalone mode — it
# never invokes rustc, so only qbc itself needs building.
cargo build --manifest-path "$ROOT/Cargo.toml" --bin qbc --release --quiet

"$QBC" "$BAS" --opt-report
