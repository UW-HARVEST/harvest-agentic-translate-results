#!/usr/bin/env bash
# Phase D driver.
#
#   1. rebuild the C reference .so and the Rust cdylib
#   2. diff `nm -D` exported symbols (must be empty)
#   3. enumerate the cargo feature combinations mechanically from Cargo.toml
#   4. run the whole differential suite once per combination
#
# Usage: scripts/check_features.sh
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE=$PWD
CSRC=$CRATE/../c_src
FAIL=0

echo "=== 1. build C reference shared object ==="
mkdir -p "$CSRC/build"
( cd "$CSRC/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
CSO=$(ls "$CSRC"/build/lib*.so | head -1)
echo "  $CSO"

echo "=== 2. enumerate feature combinations ==="
# every name declared in a [features] table (excluding "default")
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
COMBOS+=("--all-features")
COMBOS+=("")                       # default features
COMBOS+=("--no-default-features")  # nothing enabled
if [ -n "$FEATURES" ]; then
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
  done
  # full power set (bounded: this crate declares none, so this stays trivial)
  n=$(echo "$FEATURES" | wc -w)
  if [ "$n" -le 6 ]; then
    set -- $FEATURES
    total=$((1 << n))
    for ((mask = 1; mask < total; mask++)); do
      sel=""
      for ((i = 0; i < n; i++)); do
        if (( (mask >> i) & 1 )); then
          eval "name=\${$((i + 1))}"
          sel="$sel,$name"
        fi
      done
      COMBOS+=("--no-default-features --features ${sel#,}")
    done
  fi
fi
# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')
echo "  declared features: ${FEATURES:-<none>}"
echo "  combinations to verify: ${#COMBOS[@]}"

for combo in "${COMBOS[@]}"; do
  label=${combo:-"(default features)"}
  echo
  echo "=== combination: $label ==="

  echo "--- build cdylib ---"
  if ! timeout 600 cargo build --release $combo >/dev/null 2>build.log; then
    echo "BUILD FAILED for $label"; tail -30 build.log; FAIL=1; continue
  fi
  RSO=$CRATE/target/release/libarr_ins_lib.so

  echo "--- nm -D symbol parity ---"
  nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u > /tmp/hv_c_syms.txt
  nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u > /tmp/hv_r_syms.txt
  MISSING=$(comm -23 /tmp/hv_c_syms.txt /tmp/hv_r_syms.txt)
  if [ -n "$MISSING" ]; then
    echo "MISSING FROM RUST .so:"; echo "$MISSING"; FAIL=1
  else
    echo "  OK: $(wc -l < /tmp/hv_c_syms.txt) C symbols, 0 missing from Rust"
  fi
  UNDEF=$(nm -D --undefined-only "$RSO" | awk '{print $2}' | sed 's/@.*//' \
    | grep -v -E '^(_ITM_|_Unwind_|__cxa_|_dl_|__libc_)' \
    | grep -v -E '^(__gmon_start__|__tls_get_addr|__errno_location|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev|sysconf|qsort|getauxval)$')
  if [ -n "$UNDEF" ]; then
    echo "UNEXPECTED UNDEFINED (non-libc) SYMBOLS:"; echo "$UNDEF"; FAIL=1
  else
    echo "  OK: 0 undefined non-libc symbols"
  fi

  echo "--- differential test suite ---"
  timeout 600 cargo test --release $combo --no-fail-fast -- --test-threads=1 >test.log 2>&1
  rc=$?
  grep -E '^(     Running|test result)' test.log
  if [ "$rc" -ne 0 ]; then
    echo "TESTS FAILED for $label (cargo exit $rc)"
    grep -n -E 'FAILED|panicked at|SIGSEGV|SIGABRT|error:' test.log | head -40
    FAIL=1
  fi
  # every test binary must have reported a result (a crashed binary reports none)
  BINS=$(grep -c '^     Running' test.log)
  RES=$(grep -c '^test result' test.log)
  echo "  $BINS test binaries, $RES results"
  if [ "$BINS" -ne "$RES" ]; then
    echo "A TEST BINARY PRODUCED NO RESULT (crash?) for $label"; FAIL=1
  fi
  if grep -q 'test result: FAILED' test.log; then
    echo "TEST FAILURES for $label"; FAIL=1
  fi
done

rm -f build.log test.log
echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit $FAIL
