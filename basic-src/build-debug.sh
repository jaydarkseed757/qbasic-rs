#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
BIN_DIR="$ROOT/bin"
QBC="$ROOT/target/release/qbc"

# Start clean: remove any previously generated output so stale files (from
# renamed/removed .bas programs or old binaries) don't linger in bin/.
rm -rf "$BIN_DIR"
mkdir -p "$BIN_DIR"

# Build runtime and qbc (release) so everything is up to date
cargo build --manifest-path "$ROOT/runtime/Cargo.toml"  --quiet
cargo build --manifest-path "$ROOT/Cargo.toml" --bin qbc  --quiet

pass=0
fail=0
failed_files=()

for bas in "$SCRIPT_DIR"/*.bas; do
    name="$(basename "$bas" .bas)"
    rs="$BIN_DIR/$name.rs"
    bin="$BIN_DIR/$name"

    printf "%-30s " "$name"
    if "$QBC" "$bas" -o "$rs" 2>/tmp/qbc-err-"$name".txt; then
        echo "ok -> bin/$name"
        pass=$((pass + 1))
    else
        echo "FAILED"
        cat /tmp/qbc-err-"$name".txt | sed 's/^/    /' >&2
        fail=$((fail + 1))
        failed_files+=("$name")
    fi
done

echo ""
echo "Results: $pass passed, $fail failed"
if [ ${#failed_files[@]} -gt 0 ]; then
    echo "Failed: ${failed_files[*]}"
    exit 1
fi
