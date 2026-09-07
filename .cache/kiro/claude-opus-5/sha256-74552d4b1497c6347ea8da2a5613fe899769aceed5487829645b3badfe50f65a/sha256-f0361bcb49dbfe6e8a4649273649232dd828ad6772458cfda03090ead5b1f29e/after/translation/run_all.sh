#!/usr/bin/env bash
# Full verification run: build the C .so, then for every feature combination
# (and both cargo profiles) rebuild the Rust cdylib, diff `nm -D` against the C
# .so, and run the whole differential test suite.
#
# Usage:  cd translation && ./run_all.sh
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

echo "=== building the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations declared in Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' \
    "$CRATE/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== Cargo.toml declares no [features]; configurations are: default, --no-default-features ==="
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")
else
  n=${#FEATURES[@]}
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then sel="${sel:+$sel,}${FEATURES[b]}"; fi
    done
    COMBOS+=("FEAT:$sel")
  done
fi

run_one() {           # $1 = profile (release|debug)  $2 = combo
  local profile="$1" combo="$2"
  local bflags=() tflags=() label="$profile/$combo"

  case "$combo" in
    DEFAULT)   ;;
    NODEFAULT) bflags+=(--no-default-features); tflags+=(--no-default-features) ;;
    FEAT:*)    bflags+=(--no-default-features --features "${combo#FEAT:}")
               tflags+=(--no-default-features --features "${combo#FEAT:}") ;;
  esac
  if [ "$profile" = "release" ]; then bflags+=(--release); tflags+=(--release); fi

  echo
  echo "############################################################"
  echo "### $label"
  echo "############################################################"

  ( cd "$CRATE" && timeout 600 cargo build "${bflags[@]}" 2>&1 | tail -3 ) || { echo "BUILD FAILED: $label"; FAIL=1; return; }

  local so="$CRATE/target/$profile/libhelxo_lib.so"
  if [ ! -f "$so" ]; then echo "MISSING $so"; FAIL=1; return; fi

  echo "--- nm -D symbol diff (C .so  ->  Rust .so) ---"
  local missing
  missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
      <(nm -D --defined-only "$so"   | awk '{print $3}' | sort))"
  if [ -n "$missing" ]; then
    echo "MISSING FROM RUST .so:"; echo "$missing"; FAIL=1
  else
    echo "OK: 0 symbols missing ($(nm -D --defined-only "$C_SO" | wc -l) C symbols all present)"
  fi
  echo "--- undefined non-libc symbols in the Rust .so ---"
  nm -D -u "$so" | awk '{print $2}' | grep -v '^$' \
    | grep -vE '^(_Unwind_|__|_ITM_|pthread_|abort|calloc|malloc|free|realloc|posix_memalign|mem(cpy|move|set|cmp)|bcmp|str(cmp|len)|printf|sprintf|write|writev|read|readlink|realpath|open64|close|lseek64|fstat64|stat64|statx|mmap64|munmap|getcwd|getenv|syscall|dl_iterate_phdr|gettid)' \
    | sed 's/^/  UNEXPECTED: /' | tee /tmp/unexp_$$.txt
  if [ -s /tmp/unexp_$$.txt ]; then FAIL=1; fi
  rm -f /tmp/unexp_$$.txt

  echo "--- differential test suite ---"
  ( cd "$CRATE" && HARVEST_RUST_SO="$so" timeout 900 cargo test "${tflags[@]}" -- --test-threads=1 ) >"/tmp/t_$$.log" 2>&1
  grep -E '^(test result|error)' "/tmp/t_$$.log"
  if grep -qE 'test result: FAILED|^error' "/tmp/t_$$.log"; then
    echo "TESTS FAILED: $label"
    grep -E '^(test .* FAILED|---- )' "/tmp/t_$$.log" | head -40
    tail -40 "/tmp/t_$$.log"
    FAIL=1
  else
    echo "OK: all tests passed for $label"
  fi
  rm -f "/tmp/t_$$.log"
}

for profile in release debug; do
  for combo in "${COMBOS[@]}"; do
    run_one "$profile" "$combo"
  done
done

echo
echo "############################################################"
if [ "$FAIL" -eq 0 ]; then echo "### ALL CONFIGURATIONS PASSED"; else echo "### FAILURES PRESENT"; fi
echo "############################################################"
exit "$FAIL"
