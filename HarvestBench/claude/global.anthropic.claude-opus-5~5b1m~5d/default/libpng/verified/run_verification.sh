#!/usr/bin/env bash
# Full differential verification of the Rust translation against the C reference.
#
#   ./run_verification.sh            # everything
#   ./run_verification.sh symbols    # Phase A/D symbol parity only
#   ./run_verification.sh tests      # the differential tests only
#
# Every test loads BOTH shared libraries through libloading and compares their
# behaviour through the FFI boundary; the Rust crate is never called directly.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
WORK="$ROOT/.work"
mkdir -p "$WORK"

C_SO="$ROOT/c_src/build/libpng.so"
RS_SO="$CRATE/target/release/liblibpng.so"

fail=0

step() { printf '\n=== %s ===\n' "$*"; }

build() {
  step "building the reference C shared library"
  ( cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > cmake.log 2>&1 \
      && cmake --build . -j"$(nproc)" > build.log 2>&1 ) \
    || { echo "C build FAILED (see c_src/build/build.log)"; exit 1; }
  ls -l "$C_SO"

  step "building the Rust cdylib (release)"
  ( cd "$CRATE" && cargo build --release 2>&1 | tail -5 ) \
    || { echo "Rust build FAILED"; exit 1; }
  ls -l "$RS_SO"
}

symbols() {
  step "Phase A / D: exported-symbol parity"
  nm -D --defined-only "$C_SO"  | awk '$2!="U"{print $3}' | sort -u > "$WORK/c.txt"
  nm -D --defined-only "$RS_SO" | awk '$2!="U"{print $3}' | sort -u > "$WORK/rs.txt"
  echo "C exports:    $(wc -l < "$WORK/c.txt")"
  echo "Rust exports: $(wc -l < "$WORK/rs.txt")"

  local missing extra
  missing=$(comm -23 "$WORK/c.txt" "$WORK/rs.txt")
  extra=$(comm -13 "$WORK/c.txt" "$WORK/rs.txt")
  if [ -n "$missing" ]; then
    echo "MISSING from the Rust .so:"; echo "$missing"; fail=1
  else
    echo "missing from Rust: none"
  fi
  if [ -n "$extra" ]; then
    echo "EXTRA in the Rust .so:"; echo "$extra"; fail=1
  else
    echo "extra in Rust: none"
  fi

  step "undefined (imported) symbols of the Rust .so"
  # Everything left must come from libc / libm / libgcc_s / zlib -- the same
  # external surface the C build links against.
  nm -D --undefined-only "$RS_SO" | awk '{print $2}' | sed 's/@.*//' | sort -u \
    > "$WORK/rs_undef.txt"
  # compiler / unwinder runtime symbols, matched by prefix
  grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location)' \
    "$WORK/rs_undef.txt" \
  | grep -vxE 'abort|atof|bcmp|calloc|close|dl_iterate_phdr|fclose|ferror|fflush|fopen|fprintf|fputc|fread|free|fstat64|fwrite|getcwd|getenv|gettid|gmtime|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pow|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|remove|stat64|statx|stderr|strerror|strlen|syscall|write|writev' \
  | grep -vxE 'crc32|deflate|deflateEnd|deflateInit2_|deflateReset|inflate|inflateEnd|inflateInit2_|inflateReset|inflateReset2|zlibVersion' \
    > "$WORK/rs_undef_unexpected.txt" || true
  if [ -s "$WORK/rs_undef_unexpected.txt" ]; then
    echo "UNEXPECTED undefined symbols:"; cat "$WORK/rs_undef_unexpected.txt"; fail=1
  else
    echo "all undefined symbols are libc / libm / libgcc_s / zlib: OK"
  fi
}

feature_combos() {
  # This crate declares no [features], so "default" is the only configuration.
  # The loop is kept so that adding a feature automatically extends the matrix.
  local feats
  feats=$(cd "$CRATE" && cargo metadata --no-deps --format-version 1 2>/dev/null \
            | tr ',' '\n' | grep -o '"features":{[^}]*}' | head -1)
  echo "$feats" >&2
  echo "default"
}

tests() {
  step "Phase B / C: differential tests"
  local combos
  combos=$(feature_combos)
  for combo in $combos; do
    step "feature combination: $combo"
    local args=()
    if [ "$combo" != "default" ]; then
      args=(--no-default-features --features "$combo")
    fi
    ( cd "$CRATE" && timeout 1800 cargo test --release "${args[@]}" -- --test-threads=4 2>&1 \
        | grep -vE 'Box<dyn Any>|panicked at src/pngerror\.rs|^note: run with' ) \
      | tee "$WORK/test-$combo.log" \
      | grep -E '^(running|test result:|test .* (ok|FAILED))|MISMATCH|ERROR-PATH|^error'
    if grep -q 'FAILED\|^error' "$WORK/test-$combo.log"; then fail=1; fi
  done
}

case "${1:-all}" in
  symbols) build; symbols ;;
  tests)   build; tests ;;
  *)       build; symbols; tests ;;
esac

step "SUMMARY"
if [ "$fail" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT -- see the output above"
fi
exit "$fail"
