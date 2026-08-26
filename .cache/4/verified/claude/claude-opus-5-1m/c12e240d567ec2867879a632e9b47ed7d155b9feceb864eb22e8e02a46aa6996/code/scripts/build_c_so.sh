#!/usr/bin/env bash
# Builds the C reference twice:
#   1. the executable, exactly as c_src/CMakeLists.txt describes it, and
#   2. a shared object with the same translation unit, so the differential
#      tests can dlopen it next to the Rust cdylib.
#
# Nothing inside c_src/ is modified: the cmake build tree lives in
# c_src/build (created by cmake itself) and the .so lands in target/cdiff/.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here"

mkdir -p target/cdiff

# 1. cmake executable (reference driver)
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
# Write through temporary files and rename, so a concurrent reader never sees a
# half written artifact.
cp -f c_src/build/driver "target/cdiff/.c_driver.$$"
mv -f "target/cdiff/.c_driver.$$" target/cdiff/c_driver

# 2. shared object with the identical translation unit
cc -shared -fPIC -o "target/cdiff/.libc_driver.so.$$" c_src/src/main.c -lm
mv -f "target/cdiff/.libc_driver.so.$$" target/cdiff/libc_driver.so

echo "built:"
ls -l target/cdiff/
