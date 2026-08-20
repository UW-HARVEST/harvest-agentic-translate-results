#!/usr/bin/env bash
# Build every artifact the differential tests need:
#   c_src/build/driver                 - C executable (cmake, as documented)
#   c_src/build/libdriver_c_full.so    - C shared object, all 6 translation units
#   c_src/build/libdriver_c.so         - C shared object, library units only
#   target/release/{libdriver.so,driver}
#   target/debug/{libdriver.so,driver}
#
# The CMake project only declares an executable, so the shared objects are built
# from the same sources with `-fPIC -shared` and *no* -O flag (the cmake project
# sets no CMAKE_BUILD_TYPE, i.e. the reference build is unoptimised).
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
cd "$HERE"

echo "== cmake build of the C executable =="
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >cmake.log 2>&1 && cmake --build . >build.log 2>&1)
tail -1 c_src/build/build.log

echo "== C shared objects =="
gcc -fPIC -shared -o c_src/build/libdriver_c_full.so \
    c_src/src/main.c c_src/src/engine.c c_src/src/a.c c_src/src/b.c c_src/src/util.c c_src/src/lib.c
gcc -fPIC -shared -o c_src/build/libdriver_c.so \
    c_src/src/engine.c c_src/src/a.c c_src/src/b.c c_src/src/util.c c_src/src/lib.c
ls -l c_src/build/*.so c_src/build/driver

echo "== Rust (release + debug) =="
cargo build --release
cargo build
ls -l target/release/libdriver.so target/release/driver target/debug/libdriver.so target/debug/driver
