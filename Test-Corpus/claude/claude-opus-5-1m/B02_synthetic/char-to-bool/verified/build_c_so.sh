#!/bin/sh
# Builds the C reference code as a shared library so it can be dlopen()ed by the
# differential integration tests.  Nothing under c_src/ is modified; the build
# products are written to c_build/ in the crate root.
#
# The compiler flags intentionally mirror c_src/CMakeLists.txt (which sets no
# CMAKE_BUILD_TYPE, hence no -O flags) so the .so behaves exactly like the
# reference `driver` executable produced by cmake.
set -eu

ROOT=$(cd "$(dirname "$0")" && pwd)
OUT="$ROOT/c_build"
mkdir -p "$OUT"

gcc -shared -fPIC -o "$OUT/libcdecisions.so" "$ROOT/c_src/src/lib.c"

echo "built $OUT/libcdecisions.so"
