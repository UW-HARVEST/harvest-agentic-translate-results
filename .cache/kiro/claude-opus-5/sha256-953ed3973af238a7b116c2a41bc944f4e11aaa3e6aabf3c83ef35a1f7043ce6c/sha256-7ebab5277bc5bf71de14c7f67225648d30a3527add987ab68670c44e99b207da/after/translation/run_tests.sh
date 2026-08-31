#!/usr/bin/env bash
# Build the C reference .so and the Rust .so, check symbol parity, then run the
# differential test suite one file at a time (each with its own timeout).
#
# NOTE: `cargo test` alone does NOT refresh the cdylib artefact that the tests
# dlopen, so the explicit `cargo build --release` below is required.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
fail=0

# 1. C reference library ------------------------------------------------------
if [ ! -f "$root/c_src/build/libmujs.so" ]; then
  ( cd "$root/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . -j8 >/dev/null ) || exit 1
fi

# 2. Rust library (the artefact under test) ----------------------------------
timeout 600 cargo build --release --manifest-path "$here/Cargo.toml" || exit 1

# 3. Symbol parity -----------------------------------------------------------
nm -D --defined-only "$root/c_src/build/libmujs.so" | awk '{print $3}' | sort >/tmp/mujs_c_syms
nm -D --defined-only "$here/target/release/libmujs.so" | awk '{print $3}' | sort >/tmp/mujs_r_syms
missing=$(comm -23 /tmp/mujs_c_syms /tmp/mujs_r_syms)
if [ -n "$missing" ]; then
  echo "FAIL: exports present in the C .so but missing from the Rust .so:"
  echo "$missing"
  fail=1
else
  echo "PASS symbol parity ($(wc -l </tmp/mujs_c_syms) C exports, all present in Rust)"
fi

# 4. Tests, one integration test file at a time -------------------------------
for f in "$here"/tests/t*.rs; do
  name="$(basename "$f" .rs)"
  out=$(timeout 600 cargo test --release --manifest-path "$here/Cargo.toml" \
          --test "$name" "$@" -- --test-threads=1 2>&1)
  rc=$?
  summary=$(printf '%s\n' "$out" | grep -E '^test result:' | tail -1)
  if [ $rc -ne 0 ]; then
    echo "FAIL $name (exit $rc) ${summary}"
    printf '%s\n' "$out" | grep -vE "^thread '<unnamed>'|^Box<dyn Any>|panicked at src/" | tail -60
    fail=1
  else
    echo "PASS $name ${summary}"
  fi
done

if [ $fail -ne 0 ]; then
  echo "SUITE FAILED"
  exit 1
fi
echo "SUITE PASSED"
