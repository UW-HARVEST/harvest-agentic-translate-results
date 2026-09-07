#!/usr/bin/env bash
# Phase D — enumerate every feature combination from Cargo.toml and run the
# whole differential suite under each one, plus the nm -D symbol diff.
set -uo pipefail
cd "$(dirname "$0")"

FAIL=0

echo "===== feature enumeration ====="
# Extract feature names from the [features] table, if any.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml | grep -v '^default$' | tr -d '"' | sort -u)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features]; the only build configuration is the default."
  COMBOS=("" "--no-default-features")
else
  echo "features found: $FEATURES"
  # power set of the feature list
  FARR=($FEATURES)
  N=${#FARR[@]}
  COMBOS=("")
  for ((mask=1; mask<(1<<N); mask++)); do
    sel=""
    for ((i=0; i<N; i++)); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${FARR[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
  done
  COMBOS+=("--no-default-features")
fi

C_SO="../c_src/build/libdriver.so"
if [ ! -f "$C_SO" ]; then
  echo "building the C shared library..."
  ( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "===== combo: $label ====="

  echo "--- cargo check ---"
  # shellcheck disable=SC2086
  timeout 600 cargo check --release $combo 2>&1 | tail -2 || FAIL=1

  echo "--- cargo build --release ---"
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $combo 2>&1 | tail -1 || FAIL=1

  echo "--- nm -D symbol diff (C vs Rust) ---"
  nm -D --defined-only "$C_SO"                 | awk '{print $3}' | sort -u > /tmp/sym_c.txt
  nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u > /tmp/sym_r.txt
  MISSING=$(comm -23 /tmp/sym_c.txt /tmp/sym_r.txt)
  if [ -n "$MISSING" ]; then
    echo "MISSING FROM RUST .so:"; echo "$MISSING"; FAIL=1
  else
    echo "OK: 0 symbols missing from the Rust .so ($(wc -l < /tmp/sym_c.txt) C symbol(s))"
  fi
  echo "--- undefined non-libc symbols in the Rust .so ---"
  UND=$(nm -D -u target/release/libdriver.so | awk '{print $2}' \
        | sed 's/@.*$//' | sort -u \
        | grep -vE '^(_ITM_|__gmon_start__|__cxa_|_Unwind_|__tls_|__pthread|pthread_|__errno|__libc|__assert|__stack_chk|__rust_probestack)' \
        | grep -vE '^(malloc|free|calloc|realloc|reallocarray|posix_memalign|aligned_alloc|memcpy|memmove|memset|memcmp|bcmp|strtod|strlen|strerror_r|abort|exit|_exit|raise|signal|sigaction|sigaltstack|write|writev|read|readlink|realpath|getcwd|getenv|sysconf|gettid|syscall|open|open64|close|lseek|lseek64|stat|stat64|fstat|fstat64|lstat64|statx|mmap|mmap64|munmap|mprotect|dl_iterate_phdr|dlsym|dladdr|dlerror|dlopen|dlclose|qsort|nanosleep|sched_yield|getrandom|poll|madvise)$' \
        || true)
  if [ -n "$UND" ]; then
    echo "NON-LIBC UNDEFINED (review):"; echo "$UND"
  else
    echo "OK: all undefined symbols resolve against libc/libgcc"
  fi
  echo "--- ldd -r (unresolved) ---"
  if ldd -r target/release/libdriver.so 2>&1 | grep -i 'undefined symbol'; then
    echo "UNRESOLVED SYMBOLS"; FAIL=1
  else
    echo "OK: no unresolved symbols"
  fi

  echo "--- cargo test --release (Phase B + Phase C) ---"
  # shellcheck disable=SC2086
  timeout 600 cargo test --release $combo 2>&1 | grep -E '^(test result|error|---- )' || FAIL=1

  echo "--- cargo test (dev profile, debug assertions on) ---"
  # shellcheck disable=SC2086
  timeout 600 cargo build $combo 2>&1 | tail -1
  # shellcheck disable=SC2086
  RUST_SO_PATH="$PWD/target/debug/libdriver.so" \
    timeout 600 cargo test $combo 2>&1 | grep -E '^(test result|error|---- )' || FAIL=1
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit "$FAIL"
