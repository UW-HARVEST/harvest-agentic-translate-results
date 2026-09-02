#!/usr/bin/env bash
# Phase D driver: symbol parity + the full test suite under every feature
# combination and both profiles. Run from translation/.
set -uo pipefail
cd "$(dirname "$0")"

ROOT=$(cd .. && pwd)
C_SO=$(ls "$ROOT"/c_src/build/lib*.so)
FAIL=0

echo "=============================================================="
echo "Enumerating features declared in Cargo.toml"
echo "=============================================================="
# Extract feature names from a [features] table, if any.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml | grep -v '^default$')

if [ -z "$FEATURES" ]; then
  echo "No [features] table -> exactly one configuration."
  COMBOS=("--no-default-features" "" "--all-features")
else
  echo "Declared features: $FEATURES"
  # Power set of the declared features.
  COMBOS=("--no-default-features" "" "--all-features")
  names=($FEATURES)
  n=${#names[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${names[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $combo")
  done
fi

for PROFILE in "" "--release"; do
  for COMBO in "${COMBOS[@]}"; do
    LABEL="cargo test ${PROFILE:-<debug>} ${COMBO:-<default features>}"
    echo
    echo "=============================================================="
    echo "$LABEL"
    echo "=============================================================="

    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $PROFILE $COMBO >/tmp/pd_build.log 2>&1; then
      echo "BUILD FAILED"; tail -20 /tmp/pd_build.log; FAIL=1; continue
    fi

    # --- symbol parity for this exact build -----------------------------
    if [ -n "$PROFILE" ]; then RS_SO=target/release/libfloat2half_lib.so
    else RS_SO=target/debug/libfloat2half_lib.so; fi

    diff <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort) \
      > /tmp/pd_symdiff.txt
    if [ -s /tmp/pd_symdiff.txt ]; then
      echo "SYMBOL DIFF NOT EMPTY:"; cat /tmp/pd_symdiff.txt; FAIL=1
    else
      echo "symbol diff: EMPTY ($(nm -D --defined-only "$C_SO" | wc -l) exported symbol(s) matched)"
    fi

    # Any undefined symbol in the Rust .so that is not libc/libgcc.
    STRAY=$(nm -D -u "$RS_SO" | awk '{print $NF}' \
      | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^_Unwind_|^gettid$|^statx$|^__cxa_|^__tls_get_addr$|^__errno_location$' \
      | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|getcwd|getenv|malloc|memcpy|memmove|memset|posix_memalign|read|readlink|realloc|realpath|strlen|syscall|write|writev|fstat64|lseek64|mmap64|munmap|open64|stat64|pthread_key_create|pthread_key_delete|pthread_setspecific)$')
    if [ -n "$STRAY" ]; then
      echo "UNDEFINED NON-LIBC SYMBOLS:"; echo "$STRAY"; FAIL=1
    else
      echo "undefined non-libc symbols: 0"
    fi

    # --- differential test suite ---------------------------------------
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test $PROFILE $COMBO >/tmp/pd_test.log 2>&1; then
      echo "TESTS FAILED"; grep -E 'MISMATCH|panicked|test result' /tmp/pd_test.log | head -20; FAIL=1
    else
      grep -E 'test result' /tmp/pd_test.log
      grep -E 'exhaustive sweep' /tmp/pd_test.log || true
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "PHASE D: ALL CONFIGURATIONS PASSED"
else
  echo "PHASE D: FAILURES PRESENT"
fi
exit "$FAIL"
