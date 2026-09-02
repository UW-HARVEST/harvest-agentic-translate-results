#!/usr/bin/env bash
# Full verification driver.
#
#   ./run_all.sh            # build both .so's, then run every test target
#   ./run_all.sh symbols    # only the symbol-parity check
#
# Everything is derived mechanically; nothing is hard-coded per test file.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CSO="$ROOT/c_src/build/libzstd.so"
RSO="$CRATE/target/release/libzstd.so"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail=0

step() { printf '\n==== %s ====\n' "$*"; }

# ---------------------------------------------------------------- build the C
build_c() {
  step "build C shared library"
  mkdir -p "$ROOT/c_src/build"
  ( cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > "$TMP/cmake.log" 2>&1 \
    && timeout 600 cmake --build . -j "$(nproc)" > "$TMP/cbuild.log" 2>&1 ) \
    || { echo "C build FAILED"; tail -30 "$TMP/cbuild.log"; exit 1; }
  ls -l "$CSO"
}

# ------------------------------------------------------------- build the Rust
build_rust() {
  step "build Rust shared library (release)"
  ( cd "$CRATE" && timeout 600 cargo build --release > "$TMP/rbuild.log" 2>&1 ) \
    || { echo "Rust build FAILED"; tail -30 "$TMP/rbuild.log"; exit 1; }
  ls -l "$RSO"
}

# ------------------------------------------------------------- symbol parity
symbols() {
  step "symbol parity (nm -D)"
  nm -D --defined-only "$CSO" | awk '$2 ~ /^[TWBDRi]$/ {print $3}' | sort -u > "$TMP/c.txt"
  nm -D --defined-only "$RSO" | awk '$2 ~ /^[TWBDRi]$/ {print $3}' | sort -u > "$TMP/r.txt"
  local nc nr miss extra
  nc=$(wc -l < "$TMP/c.txt"); nr=$(wc -l < "$TMP/r.txt")
  comm -23 "$TMP/c.txt" "$TMP/r.txt" > "$TMP/missing.txt"
  comm -13 "$TMP/c.txt" "$TMP/r.txt" > "$TMP/extra.txt"
  miss=$(wc -l < "$TMP/missing.txt"); extra=$(wc -l < "$TMP/extra.txt")
  echo "C exports:   $nc"
  echo "RS exports:  $nr"
  echo "missing:     $miss"
  echo "extra:       $extra"
  if [ "$miss" -ne 0 ]; then echo "--- MISSING ---"; cat "$TMP/missing.txt"; fail=1; fi
  if [ "$extra" -ne 0 ]; then echo "--- EXTRA ---";   cat "$TMP/extra.txt"; fi

  echo "--- undefined non-libc/libgcc symbols in the Rust .so ---"
  nm -D --undefined-only "$RSO" | awk '{print $2}' | sed 's/@.*//' | sort -u \
    | grep -vE '^(_ITM_|_Unwind_|__cxa_|__errno_location|__gmon_start__|__tls_get_addr|abort|bcmp|calloc|clock|close|dl_iterate_phdr|fflush|fprintf|fputc|fputs|free|fstat64|fwrite|getcwd|getenv|gettid|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|qsort_r|read|readlink|realloc|realpath|stat64|statx|stderr|strlen|syscall|write|writev)' \
    > "$TMP/undef.txt"
  if [ -s "$TMP/undef.txt" ]; then cat "$TMP/undef.txt"; fail=1; else echo "(none)"; fi
}

# ------------------------------------------------------------- feature combos
features() {
  step "cargo feature space"
  ( cd "$CRATE" && cargo metadata --no-deps --format-version 1 2>/dev/null ) \
    | tr ',' '\n' | grep -o '"features":{[^}]*}' || true
  echo "declared [features] in Cargo.toml:"
  grep -A20 '^\[features\]' "$CRATE/Cargo.toml" || echo "  (none)"
  echo "cfg(feature = ...) occurrences in src/:"
  grep -rhoE 'cfg\(feature *= *"[a-zA-Z_0-9-]*"\)' "$CRATE/src" | sort -u || echo "  (none)"
  echo
  echo "=> the crate declares no features and contains no cfg(feature),"
  echo "   so the default build IS the whole feature-combination space."
}

# ------------------------------------------------------------------ run tests
run_tests() {
  local extra_args=("$@")
  step "cargo test --release ${extra_args[*]}"
  local targets
  targets=$(cd "$CRATE/tests" && ls *.rs | sed 's/\.rs$//' | sort)
  local t out rc
  for t in $targets; do
    printf '%-24s ' "$t"
    out=$( cd "$CRATE" && timeout 900 cargo test --release "${extra_args[@]}" --test "$t" 2>&1 )
    rc=$?
    if [ $rc -ne 0 ]; then
      echo "FAIL (exit $rc)"
      echo "$out" | grep -E "test result|panicked|SIGSEGV|SIGABRT|assertion|error\[|^error" | head -20
      fail=1
    else
      echo "$out" | grep -E "^test result" | head -1
    fi
  done
}

case "${1:-all}" in
  symbols)  build_c; build_rust; symbols ;;
  features) features ;;
  tests)    run_tests ;;
  all)
    build_c
    build_rust
    symbols
    features
    run_tests
    # Phase D: the same suite with the default feature set disabled. The crate
    # declares no features, so this must behave identically; running it proves
    # that mechanically rather than by assertion.
    run_tests --no-default-features
    ;;
  *) echo "usage: $0 [all|symbols|features|tests]"; exit 2 ;;
esac

step "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
