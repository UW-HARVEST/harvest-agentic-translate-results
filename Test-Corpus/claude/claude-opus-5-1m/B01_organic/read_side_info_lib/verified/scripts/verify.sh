#!/usr/bin/env bash
# Full verification pipeline:
#   1. build the C reference as a shared object
#   2. build the Rust cdylib (dev and release)
#   3. cargo check/test for every feature combination
#   4. re-run the whole differential suite against the *release* Rust .so
#   5. print the nm -D symbol diff
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
rc=0

echo "=================== 1. build C reference ==================="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=$ROOT/c_src/build/libtranslated_rust.so
ls -l "$C_SO"

echo "=================== 2. build Rust cdylib ==================="
cargo build            || rc=1
cargo build --release  || rc=1

echo "=================== 3. feature combinations ==================="
bash scripts/check_all_features.sh || rc=1

echo "=================== 4. suite vs RELEASE .so ==================="
RUST_SO=$ROOT/target/release/libread_side_info_lib.so \
    cargo test -- --test-threads=4 || rc=1

echo "=================== 5. nm -D symbol diff ==================="
diff <(nm -D --defined-only "$C_SO"                        | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/debug/libread_side_info_lib.so | awk '{print $NF}' | sort) \
     && echo "debug .so: symbol sets identical" || { echo "SYMBOL DIFF (debug)"; rc=1; }
diff <(nm -D --defined-only "$C_SO"                          | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/release/libread_side_info_lib.so | awk '{print $NF}' | sort) \
     && echo "release .so: symbol sets identical" || { echo "SYMBOL DIFF (release)"; rc=1; }

echo
[ $rc -eq 0 ] && echo "===== VERIFICATION COMPLETE: ALL GREEN =====" \
              || echo "===== VERIFICATION FAILED ====="
exit $rc
