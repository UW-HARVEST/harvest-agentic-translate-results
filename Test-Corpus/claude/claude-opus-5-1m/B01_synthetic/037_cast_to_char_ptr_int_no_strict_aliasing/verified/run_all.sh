#!/usr/bin/env bash
# Full verification run:
#   1. build the C code (CMake executable + shared library)
#   2. cargo check every feature combination (check_features.sh)
#   3. build + test the Rust side under every Cargo profile
#   4. symbol parity (Phase D)
set -u
cd "$(dirname "$0")"
rc=0
step() { echo; echo "############ $* ############"; }

step "1. build the C code"
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . ) \
  || { echo "FAIL: cmake build"; rc=1; }
mkdir -p build_c
gcc -shared -fPIC -fno-strict-aliasing -O0 c_src/src/main.c -o build_c/libcdriver.so \
  || { echo "FAIL: C shared library"; rc=1; }
ls -l c_src/build/driver build_c/libcdriver.so

step "2. cargo check, every feature combination"
./check_features.sh || rc=1

step "3. build + differential tests, every profile"
for prof in dev release; do
  flag=""; dir="debug"
  if [ "$prof" = "release" ]; then flag="--release"; dir="release"; fi
  echo "--- cargo build $flag ---"
  timeout 600 cargo build --offline $flag || { echo "FAIL: cargo build $flag"; rc=1; continue; }
  # the crate declares no features, so the only combinations are
  # {} == --no-default-features and the (identical) default set
  for feat in "--no-default-features" ""; do
    echo "--- cargo test $flag $feat ---"
    timeout 600 cargo test --offline $flag $feat -- --test-threads=4 \
      || { echo "FAIL: cargo test $flag $feat"; rc=1; }
  done
  echo "--- symbol parity ($dir) ---"
  ./symbol_parity.sh "$dir" || rc=1
done

step "RESULT"
if [ "$rc" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT (rc=$rc)"; fi
exit $rc
