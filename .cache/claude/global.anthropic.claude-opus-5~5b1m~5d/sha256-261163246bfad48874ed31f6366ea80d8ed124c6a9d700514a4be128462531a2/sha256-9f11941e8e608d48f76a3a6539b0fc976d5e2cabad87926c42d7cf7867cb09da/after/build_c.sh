#!/bin/bash
# Build the C code (shared library + driver executable) for a given OP/REPEAT.
# Usage: ./build_c.sh <OP> <REPEAT>   e.g. ./build_c.sh add 5
set -euo pipefail
OP="${1:-add}"
REPEAT="${2:-5}"
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/cbuild/${OP}_${REPEAT}"
mkdir -p "$OUT"

# Shared library: mdcore.c only (mdmain.c has main()).
gcc -O2 -fPIC -shared -DOP="$OP" -DREPEAT="$REPEAT" \
    -I"$ROOT/c_src/src" \
    -o "$OUT/libdriver_c.so" "$ROOT/c_src/src/mdcore.c"

# Driver executable via CMake, as documented in c_src/CMakeLists.txt.
CB="$OUT/cmake"
mkdir -p "$CB"
cmake -S "$ROOT/c_src" -B "$CB" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      -DOP="$OP" -DREPEAT="$REPEAT" > /dev/null
cmake --build "$CB" --clean-first > /dev/null
cp "$CB/driver" "$OUT/driver_c"
echo "built $OUT/libdriver_c.so and $OUT/driver_c"
