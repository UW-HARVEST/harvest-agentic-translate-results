#!/usr/bin/env bash
# Builds the C .so, builds the Rust cdylib (so the tests never load a stale
# library) and runs the whole differential suite.
#
#   scripts/run_tests.sh [--release] [extra cargo flags...]
set -uo pipefail
cd "$(dirname "$0")/.."

# --- C reference library -------------------------------------------------
if [ ! -f ../c_src/build/libdriver.so ]; then
  (cd ../c_src && mkdir -p build && cd build \
     && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
     && cmake --build . >/dev/null) || exit 1
fi

# --- Rust cdylib (must be relinked before `cargo test`) ------------------
# Cargo.lock is complete and libloading is the only dev-dependency, so this
# works without network access as long as the crate cache is populated. Fall
# back to --offline if the environment refuses network egress mid-resolve.
cargo build "$@" || cargo build --offline "$@" || exit 1
cargo test  "$@" -- --test-threads=4
