#!/usr/bin/env bash
# Build the C sources as a shared library for one (OP, REPEAT) configuration.
#
# c_src/CMakeLists.txt builds an *executable* from src/mdcore.c + src/mdmain.c
# with CMAKE_C_FLAGS="-DOP=${OP} -DREPEAT=${REPEAT}" (no optimization flags,
# since no CMAKE_BUILD_TYPE is set). This script compiles the very same two
# translation units with the very same defines into a position-independent
# shared object so the exported surface can be dlopen'd.
#
# usage: build_c_so.sh <OP> <REPEAT> [extra cflags...]
# prints the path of the produced .so
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OP="${1:-add}"
REPEAT="${2:-5}"
shift 2 || true
OUT="$ROOT/cbuild/so/libdriver_${OP}_${REPEAT}.so"
mkdir -p "$ROOT/cbuild/so"
gcc -shared -fPIC -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    "-DOP=${OP}" "-DREPEAT=${REPEAT}" "$@" \
    -o "$OUT" "$ROOT/c_src/src/mdcore.c" "$ROOT/c_src/src/mdmain.c"
echo "$OUT"
