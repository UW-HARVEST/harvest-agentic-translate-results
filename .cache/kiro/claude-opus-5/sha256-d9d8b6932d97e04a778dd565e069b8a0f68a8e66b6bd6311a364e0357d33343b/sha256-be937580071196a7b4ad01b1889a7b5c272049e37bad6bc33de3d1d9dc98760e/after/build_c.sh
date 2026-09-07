#!/usr/bin/env bash
# Build the C side (shared library + driver executable) for a given OP/REPEAT.
# Nothing is written into c_src/ -- all artifacts land in cbuild/.
set -euo pipefail

OP="${1:-add}"
REPEAT="${2:-5}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$ROOT/c_src/src"
OUT="$ROOT/cbuild/${OP}_${REPEAT}"
mkdir -p "$OUT"

CFLAGS=(-DOP="$OP" -DREPEAT="$REPEAT" -fPIC -I"$SRC")

# Shared library: exactly the translation unit(s) that mdcore.c contributes.
gcc "${CFLAGS[@]}" -shared -o "$OUT/libmdcore.so" "$SRC/mdcore.c"

# Driver executable: mirrors add_executable(driver src/mdcore.c src/mdmain.c)
gcc "${CFLAGS[@]}" -o "$OUT/driver" "$SRC/mdcore.c" "$SRC/mdmain.c"

echo "built $OUT/libmdcore.so and $OUT/driver (OP=$OP REPEAT=$REPEAT)"
