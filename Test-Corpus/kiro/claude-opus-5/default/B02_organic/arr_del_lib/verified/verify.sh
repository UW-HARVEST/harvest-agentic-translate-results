#!/usr/bin/env bash
# One-shot Phase A-D verification: build the C .so, build the Rust .so, check
# symbol parity, and run the differential test suite for every feature
# combination (plus a heap-checked pass and a debug-profile pass).
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
rc=0

echo "############ 1. build the C shared library ############"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
  && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
ls -la "$ROOT"/c_src/build/lib*.so
echo

echo "############ 2. cargo check ############"
timeout 600 cargo check --release 2>&1 | grep -E "^(error|warning: unused)" && rc=1
echo "  no errors"
echo

echo "############ 3. Phase D: symbol parity + all feature combinations ############"
timeout 600 ./check_features.sh || rc=1
echo

echo "############ 4. heap-checked pass (MALLOC_CHECK_=3, MALLOC_PERTURB_) ############"
MALLOC_CHECK_=3 MALLOC_PERTURB_=170 timeout 600 cargo test --release 2>&1 \
    | grep -E "^test result:|FAILED|panicked" || rc=1
echo

echo "############ 5. debug-profile Rust .so (overflow checks ON) ############"
timeout 600 cargo build > /dev/null 2>&1
RUST_SO="$PWD/target/debug/libarr_del_lib.so" timeout 600 cargo test --release 2>&1 \
    | grep -E "^test result:|FAILED|panicked" || rc=1
echo

if [ $rc -eq 0 ]; then
    echo "=========== VERIFICATION COMPLETE: all phases pass ==========="
else
    echo "=========== VERIFICATION FAILED ==========="
fi
exit $rc
