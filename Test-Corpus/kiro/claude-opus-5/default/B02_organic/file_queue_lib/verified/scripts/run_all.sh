#!/usr/bin/env bash
# Full differential verification run.
#
#  1. build the C shared library
#  2. cargo check every feature combination (Phase D)
#  3. run the whole differential suite against the DEBUG .so
#     (overflow checks + debug assertions ON)
#  4. run it again against the RELEASE .so (the shipped artifact)
#
# Several rows deliberately pay a 5 s file_sleep(); the suite is serialized
# (--test-threads=1) because the library hardcodes the relative path
# "alerts.log" and the harness redirects fd 2.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)
fail=0

echo "############ 1. build the C shared library ############"
(cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
ls -l ../c_src/build/libdriver.so

echo
echo "############ 2. feature combinations ############"
./scripts/check_features.sh || fail=1

echo
echo "############ 3. build both Rust profiles ############"
cargo build || exit 1
cargo build --release || exit 1
ls -l target/debug/libdriver.so target/release/libdriver.so

for profile in debug release; do
  echo
  echo "############ 4. differential suite vs $profile .so ############"
  log="/tmp/driver-diff-$profile.log"
  DRIVER_RUST_SO="$ROOT/target/$profile/libdriver.so" \
    timeout 600 cargo test -- --test-threads=1 >"$log" 2>&1
  rc=$?
  grep -E '^(running|test result:)|^test .* (ok|FAILED|ignored)$' "$log" \
    | grep -vE '^test .* ok$'
  echo "--- per-target results ($profile) ---"
  grep -E '^     Running|^test result:' "$log" | sed 's/^ *//'
  passed=$(grep -oP 'test result: \w+\. \K[0-9]+' "$log" | awk '{s+=$1} END {print s+0}')
  failed=$(grep -oP '(?<=; )\K[0-9]+(?= failed)' "$log" | awk '{s+=$1} END {print s+0}')
  echo "TOTAL $profile: ${passed:-0} passed, ${failed:-0} failed (cargo rc=$rc), log=$log"
  if [ "$rc" != "0" ] || [ "${failed:-0}" != "0" ]; then
    echo "SUITE FAILED for profile=$profile"
    grep -A12 '^failures:' "$log" | head -60
    fail=1
  fi
done

echo
echo "############ symbol diff ############"
diff <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort) \
     | grep '^<' && { echo "MISSING SYMBOLS IN RUST"; fail=1; } || echo "symbol diff: empty (OK)"

echo
if [ "$fail" = 0 ]; then echo "ALL GREEN"; else echo "FAILURES PRESENT"; fi
exit $fail
