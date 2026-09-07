#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
#
#   ./verify.sh
#
# Rebuilds the C .so and the Rust .so, diffs `nm -D`, then runs the whole
# differential test suite under every feature combination declared in
# Cargo.toml (plus --no-default-features and --all-features).
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
FAIL=0

echo "=== 1. build C shared library ==============================="
mkdir -p "$CBUILD"
( cd "$CBUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
C_SO="$(ls "$CBUILD"/*.so | head -1)"
echo "C   .so: $C_SO"

echo
echo "=== 2. enumerate feature combinations ======================="
# Mechanically extract the [features] table from Cargo.toml.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {sub(/[[:space:]]*=.*/,""); print}
' "$CRATE/Cargo.toml")

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares NO [features] table -> exactly one configuration."
  COMBOS=("<default>" "<none>")
else
  FA=($FEATURES)
  N=${#FA[@]}
  echo "features: ${FA[*]}"
  COMBOS=("<default>" "<none>")
  # full power set
  for ((mask=1; mask<(1<<N); mask++)); do
    combo=""
    for ((i=0; i<N; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FA[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
  COMBOS+=("<all>")
fi
printf 'combination: %s\n' "${COMBOS[@]}"

echo
echo "=== 3. per-combination build + symbol diff + tests =========="
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    "<default>") FLAGS=() ;;
    "<none>")    FLAGS=(--no-default-features) ;;
    "<all>")     FLAGS=(--all-features) ;;
    *)           FLAGS=(--no-default-features --features "$combo") ;;
  esac

  echo
  echo "--- combination: $combo   flags: ${FLAGS[*]:-none} ---"

  ( cd "$CRATE" && timeout 600 cargo build --release "${FLAGS[@]}" 2>&1 | tail -3 ) \
    || { echo "RUST BUILD FAILED for $combo"; FAIL=1; continue; }

  R_SO="$CRATE/target/release/libarr_push_lib.so"

  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/r_syms.txt
  MISSING=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
  EXTRA=$(comm -13 /tmp/c_syms.txt /tmp/r_syms.txt)
  echo "C exports: $(wc -l < /tmp/c_syms.txt)   Rust exports: $(wc -l < /tmp/r_syms.txt)"
  if [ -n "$MISSING" ]; then echo "MISSING FROM RUST:"; echo "$MISSING"; FAIL=1;
  else echo "symbol diff: EMPTY (0 missing)"; fi
  if [ -n "$EXTRA" ]; then echo "EXTRA IN RUST:"; echo "$EXTRA"; FAIL=1; fi

  # Undefined symbols must all come from libc or the Rust runtime.
  # Prefix-matched runtime groups, then exact libc function names.
  UNDEF=$(nm -D -u "$R_SO" | awk '{print $2}' | sed 's/@.*//' | sort -u \
    | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__errno_location|__tls_get_addr|pthread_)' \
    | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|memcmp|mmap64|munmap|open64|posix_memalign|read|readlink|realloc|realpath|sprintf|stat64|statx|strcmp|strlen|syscall|write|writev)$')
  if [ -n "$UNDEF" ]; then echo "UNEXPECTED UNDEFINED SYMBOLS:"; echo "$UNDEF"; FAIL=1;
  else echo "undefined non-libc symbols: 0"; fi

  ( cd "$CRATE" && timeout 900 cargo test --release "${FLAGS[@]}" --tests \
      -- --test-threads=1 2>&1 | grep -E '^test result|^error|FAILED' ) \
    || { echo "TESTS FAILED for $combo"; FAIL=1; }
done

echo
echo "=== 4. binary executables =================================="
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  echo "C CMakeLists declares an executable -- stdout comparison REQUIRED"; FAIL=1
else
  echo "c_src/CMakeLists.txt: no add_executable -> library only, no driver binary."
fi
if [ -f "$CRATE/src/main.rs" ] || grep -q '^\[\[bin\]\]' "$CRATE/Cargo.toml"; then
  echo "Rust crate declares a binary -- stdout comparison REQUIRED"; FAIL=1
else
  echo "translation/Cargo.toml: crate-type = cdylib+rlib, no [[bin]] and no src/main.rs."
fi

echo
if [ "$FAIL" -eq 0 ]; then echo "=== ALL PHASE D CHECKS PASSED ==="; else echo "=== FAILURES PRESENT ==="; fi
exit $FAIL
