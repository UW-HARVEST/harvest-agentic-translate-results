#!/usr/bin/env bash
# Phase D driver: symbol parity + the full test suite under every feature
# combination and both profiles.  Run from the crate root (translation/).
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
cd "$CRATE" || exit 1
fail=0

echo "=== 1. build C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null 2>&1 ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)
echo "C  .so: $C_SO"

echo
echo "=== 2. enumerate feature combinations ==="
# Every feature declared in Cargo.toml (empty list => only the default config).
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,"");print}' Cargo.toml)
if [ -z "$FEATS" ]; then
  echo "no [features] table -> configurations: default, --no-default-features"
  COMBOS=("default" "none")
else
  echo "features: $FEATS"
  COMBOS=("default" "none")
  for f in $FEATS; do COMBOS+=("$f"); done
  COMBOS+=("$(echo $FEATS | tr ' ' ',')")
fi

run_combo() {
  local label="$1"; shift
  local profile="$1"; shift
  local -a flags=()
  case "$label" in
    default) ;;
    none)    flags+=(--no-default-features) ;;
    *)       flags+=(--no-default-features --features "$label") ;;
  esac
  [ "$profile" = "release" ] && flags+=(--release)

  echo
  echo "--- combo: features=$label profile=$profile ---"
  cargo build --offline "${flags[@]}" >/dev/null 2>&1 \
    || { echo "  BUILD FAILED"; fail=1; return; }

  local RS_SO="target/$profile/libbetagamma_lib.so"
  if [ ! -f "$RS_SO" ]; then echo "  MISSING $RS_SO"; fail=1; return; fi

  # Symbol parity: every symbol the C .so exports must be exported by Rust.
  local missing
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO"  | awk '$2=="T"{print $3}' | sort -u) \
    <(nm -D --defined-only "$RS_SO" | awk '$2=="T"||$2=="W"{print $3}' | sort -u))
  if [ -n "$missing" ]; then
    echo "  SYMBOL DIFF NOT EMPTY -- missing from Rust .so:"; echo "$missing" | sed 's/^/    /'
    fail=1
  else
    echo "  symbol diff: EMPTY ($(nm -D --defined-only "$C_SO" | awk '$2=="T"' | wc -l) C symbols all present)"
  fi

  # Non-libc undefined symbols in the Rust .so.
  local undef
  undef=$(nm -D --undefined-only "$RS_SO" | awk '$1=="U"{print $2}' \
    | sed 's/@.*//' \
    | grep -vE '^(malloc|calloc|realloc|free|posix_memalign|strcpy|strlen|memcpy|memmove|memset|bcmp|abort|getenv|getcwd|readlink|realpath|open64|close|read|write|writev|lseek64|fstat64|stat64|statx|mmap64|munmap|syscall|dl_iterate_phdr|__errno_location|__tls_get_addr|__cxa_[a-z_]*|_Unwind_[A-Za-z]*|pthread_[a-z_]*|gettid|_ITM_[A-Za-z]*|__gmon_start__)$')
  if [ -n "$undef" ]; then
    echo "  UNRESOLVED non-libc symbols:"; echo "$undef" | sed 's/^/    /'; fail=1
  else
    echo "  undefined non-libc symbols: 0"
  fi

  local out
  out=$(timeout 600 cargo test --offline "${flags[@]}" 2>&1)
  echo "$out" | grep -E "^test result:" | sed 's/^/  /'
  if echo "$out" | grep -q "test result: FAILED\|error\[|error:"; then
    echo "  TESTS FAILED"; echo "$out" | grep -E "^test .*FAILED|panicked" | head -20 | sed 's/^/    /'
    fail=1
  fi
}

for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    run_combo "$combo" "$profile"
  done
done

echo
if [ "$fail" -eq 0 ]; then echo "=== ALL COMBINATIONS PASSED ==="; else echo "=== FAILURES PRESENT ==="; fi
exit "$fail"
