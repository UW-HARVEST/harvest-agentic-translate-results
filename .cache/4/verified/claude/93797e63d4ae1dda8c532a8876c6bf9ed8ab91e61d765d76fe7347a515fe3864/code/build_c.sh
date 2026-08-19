#!/usr/bin/env bash
# Builds the C reference artefacts. c_src/ itself is never modified.
#
#   c_src/build/driver        — the executable, exactly as CMakeLists.txt defines it
#   c_build/libcdriver.so     — the same translation unit as a shared object, so the
#                               exported `run` / `main` can be compared through FFI
set -euo pipefail
cd "$(dirname "$0")"

mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)

mkdir -p c_build
# Same (default, unoptimised) flags CMake uses: CMAKE_BUILD_TYPE is empty.
gcc -fPIC -shared -o c_build/libcdriver.so c_src/src/main.c

echo "C artefacts:"
ls -l c_src/build/driver c_build/libcdriver.so
