#!/bin/sh
# Build the C reference implementation (c_src, unmodified) as a shared library
# so the differential tests can dlopen it next to the Rust cdylib.
#
# The source list mirrors the CMake target exactly:
#     add_executable(driver src/main.c src/scene.c src/shape.c)
set -e
here=$(cd "$(dirname "$0")" && pwd)
out="$here/c_build"
mkdir -p "$out"
gcc -shared -fPIC -O0 -g \
    -I "$here/c_src/include" \
    "$here/c_src/src/main.c" \
    "$here/c_src/src/scene.c" \
    "$here/c_src/src/shape.c" \
    -o "$out/libcdriver.so"
echo "built $out/libcdriver.so"
