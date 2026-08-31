#!/bin/bash
# Build the C reference .so, the Rust cdylib, then run the differential tests.
# Usage: ./run_tests.sh [extra cargo test args...]
set -o pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

if [ ! -f "$ROOT/c_src/build/libpng.so" ]; then
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . -j8 >/dev/null ) || exit 1
fi

timeout 600 cargo build --release 2>&1 | grep -E "^(error|warning: unused)" && true
timeout 600 cargo build --release >/dev/null 2>&1 || { echo "RUST BUILD FAILED"; cargo build --release 2>&1 | tail -30; exit 1; }
timeout 600 cargo test --release "$@" 2>&1 | grep -vE "^warning|^ *\||^ *-->|^ *=|^[0-9]+ *\||^ *\^"
