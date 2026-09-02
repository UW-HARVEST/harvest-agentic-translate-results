#!/usr/bin/env bash
# Build the C reference .so and the Rust cdylib, then run the differential suite.
#
# `cargo test` alone does NOT build the cdylib (no test target links it, because
# the crate only produces a cdylib), so it must be built explicitly or the
# harness loads a stale artifact. tests/common/mod.rs hard-fails on staleness.
#
# Usage:
#   ./run_tests.sh                        # release profile, whole suite
#   PROFILE=debug ./run_tests.sh          # debug profile
#   FEATURES="--no-default-features" ./run_tests.sh
#   ./run_tests.sh --test phase_b_leaf    # extra args go to `cargo test` only
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
PROFILE="${PROFILE:-release}"
# Feature flags must reach BOTH `cargo build` and `cargo test`; test filters
# (--test X, -- --nocapture, ...) must reach ONLY `cargo test`.
read -r -a FEATURE_ARGS <<<"${FEATURES:-}"
TEST_ARGS=("$@")

# ---- 1. C reference library ------------------------------------------------
if ! ls "$ROOT/c_src/build"/*.so >/dev/null 2>&1; then
    echo "== building C reference =="
    mkdir -p "$ROOT/c_src/build"
    ( cd "$ROOT/c_src/build" \
      && timeout 300 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && timeout 300 cmake --build . >/dev/null )
fi
C_SO="$(ls "$ROOT/c_src/build"/*.so | head -1)"

# ---- 2. Rust cdylib (must be built explicitly) -----------------------------
cd "$HERE"
if [[ "$PROFILE" == "release" ]]; then
    PROFILE_ARGS=(--release)
    OUT=target/release
else
    PROFILE_ARGS=()
    OUT=target/debug
fi
timeout 600 cargo build "${PROFILE_ARGS[@]}" ${FEATURE_ARGS[@]+"${FEATURE_ARGS[@]}"} --lib
RUST_SO="$HERE/$OUT/libcircle_collide_lib.so"
[[ -f "$RUST_SO" ]] || { echo "FATAL: cdylib not produced at $RUST_SO" >&2; exit 1; }

echo "profile : $PROFILE   features: ${FEATURE_ARGS[*]:-<default>}"
echo "C   .so : $C_SO"
echo "Rust.so : $RUST_SO"

# ---- 3. Differential suite -------------------------------------------------
export C_SO_PATH="$C_SO"
export RUST_SO_PATH="$RUST_SO"
timeout 600 cargo test "${PROFILE_ARGS[@]}" ${FEATURE_ARGS[@]+"${FEATURE_ARGS[@]}"} \
    ${TEST_ARGS[@]+"${TEST_ARGS[@]}"}
