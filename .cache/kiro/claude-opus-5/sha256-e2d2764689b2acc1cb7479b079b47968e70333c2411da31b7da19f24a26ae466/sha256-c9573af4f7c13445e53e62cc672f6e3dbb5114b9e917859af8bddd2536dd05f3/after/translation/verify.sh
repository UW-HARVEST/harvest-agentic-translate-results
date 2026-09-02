#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
# Usage: ./verify.sh          (run from translation/)
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
CSRC="$ROOT/../c_src"
fail=0

echo "=== 1. build the C shared library ==="
mkdir -p "$CSRC/build"
( cd "$CSRC/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
CSO="$(find "$CSRC/build" -name '*.so' | head -1)"
echo "C  .so: $CSO"

echo
echo "=== 2. enumerate feature combinations from Cargo.toml ==="
# Every feature name declared under [features] (none for this crate).
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{print $1}' "$ROOT/Cargo.toml")
if [ -z "$FEATS" ]; then
  echo "no [features] declared -> the only configurations are:"
  COMBOS=("" "--no-default-features")
else
  echo "features: $FEATS"
  COMBOS=("" "--no-default-features")
  for f in $FEATS; do
    COMBOS+=("--no-default-features --features $f")
  done
  COMBOS+=("--all-features")
fi
for c in "${COMBOS[@]}"; do echo "  cargo <cmd> ${c:-<default>}"; done

echo
echo "=== 3. per-combination: check, build, symbol parity, tests ==="
for PROFILE in "" "--release"; do
  for COMBO in "${COMBOS[@]}"; do
    tag="profile='${PROFILE:-debug}' features='${COMBO:-default}'"
    echo
    echo "--- $tag ---"

    timeout 600 cargo check $PROFILE $COMBO >/dev/null 2>&1 \
      || { echo "  cargo check FAILED"; fail=1; continue; }

    timeout 600 cargo build $PROFILE $COMBO >/dev/null 2>&1 \
      || { echo "  cargo build FAILED"; fail=1; continue; }

    if [ -z "$PROFILE" ]; then RSO="$ROOT/target/debug/libarr_push_lib.so";
    else RSO="$ROOT/target/release/libarr_push_lib.so"; fi

    nm -D --defined-only "$CSO"  | awk '{print $3}' | sort > /tmp/csym.$$
    nm -D --defined-only "$RSO"  | awk '{print $3}' | sort > /tmp/rsym.$$
    missing=$(comm -23 /tmp/csym.$$ /tmp/rsym.$$)
    extra=$(comm -13 /tmp/csym.$$ /tmp/rsym.$$)
    echo "  symbols: C=$(wc -l < /tmp/csym.$$) Rust=$(wc -l < /tmp/rsym.$$)"
    if [ -n "$missing" ]; then echo "  MISSING in Rust:"; echo "$missing" | sed 's/^/    /'; fail=1
    else echo "  symbol diff: empty (OK)"; fi
    [ -n "$extra" ] && { echo "  extra in Rust:"; echo "$extra" | sed 's/^/    /'; }

    # undefined symbols that are not libc / libgcc-unwind
    nm -D --undefined-only "$RSO" | awk '{print $2}' | sed 's/@.*//' \
      | grep -vE '^(_ITM_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location|_Unwind_|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev|memcmp|sprintf|strcmp|__assert_fail)' \
      > /tmp/undef.$$
    if [ -s /tmp/undef.$$ ]; then echo "  UNEXPECTED undefined non-libc symbols:"; sed 's/^/    /' /tmp/undef.$$; fail=1
    else echo "  undefined non-libc symbols: none (OK)"; fi

    timeout 600 cargo test $PROFILE $COMBO > /tmp/test.$$ 2>&1
    rc=$?
    grep -E "^test result" /tmp/test.$$ | sed 's/^/  /'
    if [ $rc -ne 0 ]; then
      echo "  TESTS FAILED (exit $rc)"
      grep -E "panicked|FAILED|signal" /tmp/test.$$ | head -20 | sed 's/^/    /'
      fail=1
    fi
    rm -f /tmp/csym.$$ /tmp/rsym.$$ /tmp/undef.$$ /tmp/test.$$
  done
done

echo
if [ $fail -eq 0 ]; then echo "=== ALL PHASES PASSED ==="; else echo "=== FAILURES PRESENT ==="; fi
exit $fail
