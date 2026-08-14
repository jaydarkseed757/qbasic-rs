#!/usr/bin/env bash
# Transpile + compile every basic-src/*.bas program to bin/.
#
# Incremental by default: a program is SKIPPED (reusing its existing bin/
# outputs) when none of the following changed since its last successful
# build: the .bas file's mtime, the qbc binary, or the runtime rlib. A
# per-program cache entry is only written after a successful build, so a
# failing build always retries next run regardless of source changes.
#
# Usage:
#   build-all.sh            # incremental (default)
#   build-all.sh --clean    # ignore all cache state, rebuild everything

set -euo pipefail

CLEAN=0
for arg in "$@"; do
    case "$arg" in
        --clean) CLEAN=1 ;;
        *) echo "Usage: build-all.sh [--clean]" >&2; exit 1 ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
BIN_DIR="$ROOT/bin"
CACHE_DIR="$BIN_DIR/.cache"
QBC="$ROOT/target/release/qbc"
RLIB="$ROOT/target/release/libqbasic_runtime.rlib"

mkdir -p "$BIN_DIR" "$CACHE_DIR"
[ "$CLEAN" -eq 1 ] && rm -f "$CACHE_DIR"/*.meta

# Build runtime and qbc (release) so everything is up to date
cargo build --manifest-path "$ROOT/runtime/Cargo.toml" --release --quiet
cargo build --manifest-path "$ROOT/Cargo.toml" --bin qbc --release --quiet

# Portable epoch mtime (BSD `stat` on macOS, GNU `stat` on Linux).
mtime_of() {
    stat -f %m "$1" 2>/dev/null || stat -c %Y "$1" 2>/dev/null
}

qbc_hash="$(shasum -a 256 "$QBC" | cut -d' ' -f1)"
rlib_hash="$(shasum -a 256 "$RLIB" | cut -d' ' -f1)"

pass=0
cached=0
fail=0
failed_files=()

for bas in "$SCRIPT_DIR"/*.bas; do
    name="$(basename "$bas" .bas)"
    rs="$BIN_DIR/$name.rs"
    bin="$BIN_DIR/$name"
    meta="$CACHE_DIR/$name.meta"

    printf "%-30s " "$name"

    bas_mtime="$(mtime_of "$bas")"
    if [ -f "$meta" ] && [ -f "$rs" ] && [ -f "$bin" ]; then
        read -r c_mtime c_qbc c_rlib < <(tr '\n' ' ' < "$meta") || true
        if [ "$bas_mtime" = "$c_mtime" ] && [ "$qbc_hash" = "$c_qbc" ] && [ "$rlib_hash" = "$c_rlib" ]; then
            echo "cached -> bin/$name"
            cached=$((cached + 1))
            continue
        fi
    fi

    # About to (re)build: invalidate the cache entry first, so a failed
    # build never leaves behind a stale "validated" cache file.
    rm -f "$meta"

    if "$QBC" "$bas" -o "$rs" 2>/tmp/qbc-err-"$name".txt; then
        echo "ok -> bin/$name"
        pass=$((pass + 1))
        printf '%s\n%s\n%s\n' "$bas_mtime" "$qbc_hash" "$rlib_hash" > "$meta"
    else
        echo "FAILED"
        cat /tmp/qbc-err-"$name".txt | sed 's/^/    /' >&2
        fail=$((fail + 1))
        failed_files+=("$name")
    fi
done

# Prune stale outputs (and cache entries) for renamed/removed programs, so
# a program that no longer has a .bas source doesn't linger in bin/.
for f in "$BIN_DIR"/*.rs; do
    [ -e "$f" ] || continue
    name="$(basename "$f" .rs)"
    [ -f "$SCRIPT_DIR/$name.bas" ] && continue
    rm -f "$f" "$BIN_DIR/$name" "$CACHE_DIR/$name.meta"
    echo "pruned stale: $name"
done

echo ""
echo "Results: $pass passed ($cached cached), $fail failed"
if [ ${#failed_files[@]} -gt 0 ]; then
    echo "Failed: ${failed_files[*]}"
    exit 1
fi
