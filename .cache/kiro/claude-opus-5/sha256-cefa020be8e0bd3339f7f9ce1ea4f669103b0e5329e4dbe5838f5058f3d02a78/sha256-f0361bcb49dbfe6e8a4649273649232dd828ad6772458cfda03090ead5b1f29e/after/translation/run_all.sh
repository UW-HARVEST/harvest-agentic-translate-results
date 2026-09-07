#!/usr/bin/env bash
# Full verification driver: builds the C .so and the Rust cdylib, diffs their
# dynamic symbol tables, and runs every differential test under EVERY feature
# combination declared in Cargo.toml.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
fail=0

echo "=== 1. build C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && timeout 600 cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }

echo
echo "=== 2. enumerate feature combinations from Cargo.toml ==="
# Extract the [features] table; there is none here, so the combination set is
# just {default, no-default-features}. This is derived, not hardcoded.
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' Cargo.toml)
COMBOS=("" "--no-default-features")
if [ -n "$FEATS" ]; then
  for f in $FEATS; do
    COMBOS+=("--no-default-features --features $f")
    COMBOS+=("--features $f")
  done
  ALL=$(echo "$FEATS" | paste -sd, -)
  COMBOS+=("--no-default-features --features $ALL")
fi
printf 'declared features: %s\n' "${FEATS:-<none>}"
printf 'combinations to verify: %d\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '  cargo test --release %s\n' "${c:-<default>}"; done

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "=================================================================="
  echo "=== combination: $label"
  echo "=================================================================="

  echo "--- cargo check ---"
  # shellcheck disable=SC2086
  timeout 600 cargo check --release $combo 2>&1 | tail -3 || fail=1

  echo "--- build Rust cdylib ---"
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $combo 2>&1 | tail -3 || { fail=1; continue; }
  R_SO="target/release/libdriver.so"
  test -f "$R_SO" || { echo "MISSING $R_SO"; fail=1; continue; }

  echo "--- nm -D defined-symbol diff (C vs Rust) ---"
  diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort) \
    && echo "symbol diff EMPTY (parity OK)" \
    || { echo "SYMBOL DIFF NON-EMPTY"; fail=1; }

  echo "--- undefined non-libc symbols in Rust .so ---"
  bad=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' \
        | grep -vE '^(_Unwind_|_ITM_|__)' \
        | grep -vxE 'printf|putchar|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev|pthread_[a-z_]+' \
        || true)
  if [ -n "$bad" ]; then echo "UNEXPECTED: $bad"; fail=1; else echo "none"; fi

  echo "--- cargo test (Phases B, C, D) ---"
  # fd 1 is process-global for the capture harness -> single test thread.
  # shellcheck disable=SC2086
  timeout 600 cargo test --release $combo -- --test-threads=1 2>&1 \
    | grep -E '^(test |running |test result|error|failures)' | tail -60
  # shellcheck disable=SC2086
  timeout 600 cargo test --release $combo -- --test-threads=1 >/dev/null 2>&1 \
    || { echo "TESTS FAILED for $label"; fail=1; }
done

echo
echo "=================================================================="
if [ "$fail" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
