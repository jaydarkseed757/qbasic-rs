#!/bin/bash
# Launch a QBasic program in DOSBox-X via QBASIC 1.1.
# Usage: ./run.sh <program>      e.g.  ./run.sh gorilla   or   ./run.sh gorilla.bas
set -euo pipefail

DOSBOX="/Applications/dosbox-x.app/Contents/MacOS/DosBox"
QBASIC_DIR="/Users/jay/projects/MSDOS-QBASIC"
GAME_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <program>   (e.g. $0 gorilla  or  $0 gorilla.bas)" >&2
    exit 1
fi

# Accept "gorilla" or "gorilla.bas", with or without a path; keep just the name.
PROG="$(basename "$1")"
PROG="${PROG%.bas}.bas"

if [[ ! -x "$DOSBOX" ]]; then
    echo "error: DOSBox-X not found at $DOSBOX" >&2
    exit 1
fi
if [[ ! -f "$QBASIC_DIR/Qbasic.exe" ]]; then
    echo "error: Qbasic.exe not found in $QBASIC_DIR" >&2
    exit 1
fi
if [[ ! -f "$GAME_DIR/$PROG" ]]; then
    echo "error: $PROG not found in $GAME_DIR" >&2
    exit 1
fi

# -working-dir auto-mounts C: to the program directory, so QBASIC goes on E:
exec "$DOSBOX" \
    -fastlaunch \
    -working-dir "$GAME_DIR" \
    -c "mount e $QBASIC_DIR" \
    -c "c:" \
    -c "e:\\qbasic.exe /RUN c:\\$PROG"
