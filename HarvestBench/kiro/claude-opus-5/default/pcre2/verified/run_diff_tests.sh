#!/usr/bin/env bash
# Build the C reference .so and the Rust cdylib, then run the differential
# tests against the freshly built Rust shared object.
#
# Usage: ./run_diff_tests.sh [extra cargo test args...]
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

# --- C reference library -------------------------------------------------
if [ ! -f "$root/c_src/build/libpcre2.so" ]; then
  mkdir -p "$root/c_src/build"
  (cd "$root/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . -j8 >/dev/null)
fi

# --- Rust library --------------------------------------------------------
cd "$here"
timeout 600 cargo build --release 2>&1 | grep -E '^(error|warning: unused)' || true

export PCRE2_C_SO="$root/c_src/build/libpcre2.so"
export PCRE2_RUST_SO="$here/target/release/libpcre2.so"

test -f "$PCRE2_C_SO"
test -f "$PCRE2_RUST_SO"

exec timeout 900 cargo test "$@"
