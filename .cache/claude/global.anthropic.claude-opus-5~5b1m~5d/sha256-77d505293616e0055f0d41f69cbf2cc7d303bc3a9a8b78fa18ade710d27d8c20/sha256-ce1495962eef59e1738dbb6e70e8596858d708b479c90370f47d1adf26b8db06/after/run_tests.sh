#!/bin/bash
# Rebuild both shared libraries, then run the differential test suite.
# Usage: ./run_tests.sh [extra cargo test args...]
set -e
ROOT="$(cd "$(dirname "$0")" && pwd)"

# 1. C shared library
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . -j8 >/dev/null)

# 2. Rust cdylib — RELEASE (overflow-checks off, matching C wrapping arithmetic)
(cd "$ROOT/translation" && cargo build --offline --release 2>&1 \
  | grep -E '^(error|warning: unused)' || true)
test -f "$ROOT/translation/target/release/libpcre2.so"

# 3. Differential tests
cd "$ROOT/translation"
exec cargo test --offline "$@"
