#!/usr/bin/env bash
# One-shot driver: build the C shared library, build the Rust cdylib, verify
# symbol parity, then run the whole differential suite.
#
# The suite MUST run single threaded: several tests redirect fd 1 to capture the
# `printf` output of `str_dups`.
set -uo pipefail
cd "$(dirname "$0")"

echo "=== 1/5  build the C shared library ============================="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
  && cmake --build . ) || exit 1
ls -l c_src/build/*.so

echo "=== 2/5  build the Rust cdylib (release) ========================"
cargo build --offline --release || exit 1
ls -l target/release/libstr_dups_lib.so

echo "=== 3/5  symbol parity ========================================="
./check_symbols.sh || exit 1

echo "=== 4/5  feature combinations =================================="
./check_features.sh || exit 1

echo "=== 5/5  differential test suite ==============================="
cargo test --offline -- --test-threads=1
rc=$?

echo "==============================================================="
[[ $rc == 0 ]] && echo "ALL GREEN" || echo "FAILURES (rc=$rc)"
exit $rc
