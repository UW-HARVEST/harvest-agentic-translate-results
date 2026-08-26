#!/usr/bin/env bash
# Build the C reference .so, relink the Rust cdylib (`cargo test` alone does
# NOT), then run the whole differential suite.
set -euo pipefail
cd "$(dirname "$0")/.."
PROFILE_ARGS=()
[ "${1:-}" = "--release" ] && PROFILE_ARGS=(--release)

cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build c_src/build >/dev/null
cargo build --offline "${PROFILE_ARGS[@]}"
cargo test  --offline "${PROFILE_ARGS[@]}" -- --test-threads=1
