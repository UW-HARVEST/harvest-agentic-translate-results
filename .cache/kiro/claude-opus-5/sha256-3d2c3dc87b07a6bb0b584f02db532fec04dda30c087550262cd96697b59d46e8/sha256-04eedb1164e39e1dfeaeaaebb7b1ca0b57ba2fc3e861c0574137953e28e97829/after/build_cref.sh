#!/usr/bin/env bash
# Build the C reference for every OP x REPEAT configuration.
#   /tmp/cref/<op>_<repeat>/driver     - executable (cmake, mirrors CMakeLists)
#   /tmp/cref/<op>_<repeat>/libmd.so   - shared library of mdcore.c (FFI tests)
# Nothing under c_src/ is written to: cmake is invoked with an out-of-tree
# binary directory.
set -eu
ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC="$ROOT/c_src"

for op in add sub mul; do
  for r in 0 1 2 3 4 5 6 7; do
    out="/tmp/cref/${op}_${r}"
    mkdir -p "$out"
    cmake -S "$SRC" -B "$out/cmake" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
          -DOP="$op" -DREPEAT="$r" >/dev/null
    cmake --build "$out/cmake" >/dev/null
    cp "$out/cmake/driver" "$out/driver"
    gcc -O2 -fPIC -shared -DOP="$op" -DREPEAT="$r" \
        -o "$out/libmd.so" "$SRC/src/mdcore.c"
  done
done
echo "C reference built for 24 configurations in /tmp/cref"
