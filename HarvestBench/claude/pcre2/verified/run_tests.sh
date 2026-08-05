#!/bin/bash
# Build the Rust cdylib .so first (cargo test alone does NOT rebuild the
# standalone cdylib that the differential tests load via libloading), ensure
# the C .so exists, then run all differential tests.
set -e
cd "$(dirname "$0")"

# Build C .so if missing
if [ ! -f c_src/build/libpcre2.so ]; then
  (cd c_src && mkdir -p build && cd build && \
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && \
    cmake --build . >/dev/null)
fi

# Rebuild Rust cdylib so the loaded .so reflects current source
cargo build 2>&1 | tail -1

# Run tests (they load the freshly built .so)
timeout 600 cargo test "$@" 2>&1 | tail -60
