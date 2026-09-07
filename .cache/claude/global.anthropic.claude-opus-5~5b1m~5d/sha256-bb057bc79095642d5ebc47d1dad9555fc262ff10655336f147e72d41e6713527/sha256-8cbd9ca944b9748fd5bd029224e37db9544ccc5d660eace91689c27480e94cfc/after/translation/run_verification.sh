#!/usr/bin/env bash
# Full differential-verification run: builds the C .so and the Rust .so, checks
# symbol parity, then runs every test under every feature combination.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=== 1. build the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "    $C_SO"

echo "=== 2. enumerate feature combinations ==="
# every combination of the crate's own features (powerset), always including the
# no-default-features baseline.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[ ]*=/{print $1}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "    Cargo.toml declares no [features] -> a single configuration"
  COMBOS=("__default__" "__none__")
else
  COMBOS=("__default__" "__none__")
  set -- $FEATURES
  n=$#
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    i=0
    for f in $FEATURES; do
      if (( (mask >> i) & 1 )); then combo="${combo:+$combo,}$f"; fi
      i=$((i+1))
    done
    COMBOS+=("$combo")
  done
fi
printf '    combos: %s\n' "${COMBOS[*]}"

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) FLAGS=(); LABEL="default features" ;;
    __none__)    FLAGS=(--no-default-features); LABEL="--no-default-features" ;;
    *)           FLAGS=(--no-default-features --features "$combo"); LABEL="--features $combo" ;;
  esac

  echo
  echo "=== 3. [$LABEL] build the Rust shared library ==="
  cargo build --offline --release "${FLAGS[@]}" >/dev/null 2>&1 \
    || { echo "    Rust build FAILED"; FAIL=1; continue; }
  R_SO="target/release/libstr_put_lib.so"

  echo "=== 4. [$LABEL] symbol parity (nm -D) ==="
  diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
       <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort) > symdiff.txt
  if [ -s symdiff.txt ]; then
    echo "    SYMBOL DIFF NOT EMPTY:"; cat symdiff.txt; FAIL=1
  else
    echo "    0 missing, 0 extra ($(nm -D --defined-only "$C_SO" | wc -l) symbols)"
  fi
  rm -f symdiff.txt

  echo "    undefined non-libc symbols in the Rust .so:"
  nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__|_Unwind_|pthread_)' \
    | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|printf|read|readlink|realloc|realpath|sprintf|stat64|statx|strcmp|strlen|syscall|write|writev)$' \
    | sed 's/^/      /' | tee undef.txt
  if [ -s undef.txt ]; then echo "    NON-LIBC UNDEFINED SYMBOLS FOUND"; FAIL=1; else echo "      (none)"; fi
  rm -f undef.txt

  echo "=== 5. [$LABEL] differential tests ==="
  if timeout 600 cargo test --offline --release "${FLAGS[@]}" 2>&1 | tail -40; then
    :
  else
    echo "    TESTS FAILED"; FAIL=1
  fi
done

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
