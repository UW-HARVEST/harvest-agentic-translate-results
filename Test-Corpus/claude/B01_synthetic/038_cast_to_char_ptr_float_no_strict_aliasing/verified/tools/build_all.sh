#!/bin/bash
# Builds everything the differential tests need:
#   cbuild/libcdriver.so   - c_src/src/main.c compiled as a shared library
#   c_src/build/driver     - the C executable, via CMake (as documented)
#   target/<prof>/driver     - the Rust executable
#   target/<prof>/libdriver.so - the Rust cdylib
#
# Usage: tools/build_all.sh [debug|release]
set -e
cd "$(dirname "$0")/.."
ROOT=$(pwd)
PROF=${1:-release}

# ---- C shared library (kept out of target/, which `cargo clean` wipes) ----
mkdir -p cbuild
gcc -shared -fPIC -fno-strict-aliasing -o cbuild/libcdriver.so c_src/src/main.c

# ---- C executable, through the documented CMake flow ----
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)

# ---- Rust ----
if [ "$PROF" = "release" ]; then
  cargo build --release --offline
else
  cargo build --offline
fi

echo "built:"
ls -l cbuild/libcdriver.so c_src/build/driver "target/$PROF/driver" "target/$PROF/libdriver.so"
