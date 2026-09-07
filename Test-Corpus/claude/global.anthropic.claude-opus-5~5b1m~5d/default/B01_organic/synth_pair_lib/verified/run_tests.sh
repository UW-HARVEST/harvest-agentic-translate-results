#!/bin/bash
# Build the C .so and the Rust cdylib, then run the differential suite.
#
# IMPORTANT: `cargo test` alone does NOT refresh target/<profile>/libsynth_pair_lib.so,
# so the tests would load a stale library and pass vacuously. The explicit
# `cargo build` below is required; tests/common/mod.rs also asserts freshness.
set -euo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)

# 1. C shared library
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)

PROFILE="${PROFILE:-release}"
FLAGS=("--offline")
if [ "$PROFILE" = "release" ]; then FLAGS+=("--release"); fi
FLAGS+=("$@")

# 2. Rust cdylib (must precede `cargo test`)
cargo build "${FLAGS[@]}"
# 3. Differential tests
cargo test "${FLAGS[@]}"
