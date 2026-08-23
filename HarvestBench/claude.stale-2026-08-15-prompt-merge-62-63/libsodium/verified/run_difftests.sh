#!/usr/bin/env bash
# Build the C .so and the Rust cdylib, then run every differential test target.
#
# `cargo test` does NOT rebuild a cdylib-only crate for integration tests, so
# the explicit `cargo build` is MANDATORY -- otherwise the harness dlopen()s a
# stale .so and every assertion can pass vacuously. tests/common/mod.rs also
# refuses to run against a .so older than src/**.rs as a backstop.
set -uo pipefail
cd "$(dirname "$0")"

PROFILE_FLAG=""
PROFILE_DIR="debug"
if [ "${1:-}" = "--release" ]; then PROFILE_FLAG="--release"; PROFILE_DIR="release"; shift; fi

# Avoid systemd-coredump on the many intentional SIGABRTs (100x slowdown).
ulimit -c 0 2>/dev/null || true

echo "=== building C shared library ==="
if [ ! -f c_src/build/libsodium.so ]; then
  ( mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . -j "$(nproc)" >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
fi
ls -la c_src/build/libsodium.so

echo "=== building Rust cdylib ($PROFILE_DIR) ==="
cargo build $PROFILE_FLAG 2>&1 | grep -E "^(error|warning: unused)" | head -20
if [ ! -f "target/$PROFILE_DIR/liblibsodium.so" ]; then echo "RUST BUILD FAILED"; exit 1; fi
ls -la "target/$PROFILE_DIR/liblibsodium.so"

echo "=== symbol parity ==="
nm -D --defined-only c_src/build/libsodium.so | awk '{print $3}' | sort -u > /tmp/_c.txt
nm -D --defined-only "target/$PROFILE_DIR/liblibsodium.so" | awk '{print $3}' | sort -u > /tmp/_r.txt
MISSING=$(comm -23 /tmp/_c.txt /tmp/_r.txt | wc -l)
echo "C=$(wc -l < /tmp/_c.txt) Rust=$(wc -l < /tmp/_r.txt) MISSING=$MISSING"
if [ "$MISSING" -ne 0 ]; then echo "SYMBOL PARITY FAILED:"; comm -23 /tmp/_c.txt /tmp/_r.txt; exit 1; fi

echo "=== running differential tests ==="
FAIL=0
for t in $(ls tests/t*.rs | sed 's#tests/##; s#\.rs$##' | sort); do
  printf '%-24s ' "$t"
  OUT=$(timeout 900 cargo test $PROFILE_FLAG --test "$t" -- --test-threads=4 2>&1)
  RES=$(echo "$OUT" | grep -E "^test result:" | tail -1)
  if echo "$OUT" | grep -qE "^test result: ok"; then
    echo "OK   $RES"
  else
    echo "FAIL $RES"
    echo "$OUT" | grep -E "^(test .* FAILED|---- .* ----|thread .* panicked|error\[|error:)" | head -20
    FAIL=1
  fi
done
echo "=== $( [ $FAIL -eq 0 ] && echo 'ALL TEST TARGETS PASSED' || echo 'SOME TEST TARGETS FAILED' ) ==="
exit $FAIL
