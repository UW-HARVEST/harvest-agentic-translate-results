#!/usr/bin/env bash
# Differential test driver.
#
#   ./run_diff_tests.sh [extra cargo test args...]
#
# `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact, so the
# cdylib has to be built explicitly first, otherwise the integration tests
# would dlopen() a stale `.so`.  The C reference library is (re)built too.
set -euo pipefail

cd "$(dirname "$0")"

FEATURES="${FEATURES:-}"          # e.g. FEATURES="--features foo"
PROFILE_FLAGS="${PROFILE_FLAGS:-}" # e.g. PROFILE_FLAGS="--release"

# 1. C reference shared library.
if [ ! -f c_src/build/libtranslated_rust.so ]; then
    mkdir -p c_src/build
    (cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
fi

# 2. Rust cdylib (same profile the test harness will look in).
cargo build --offline $PROFILE_FLAGS $FEATURES

# 3. Differential tests.
cargo test --offline $PROFILE_FLAGS $FEATURES "$@"
