#!/bin/bash
# Run the differential test suite for every feature combination (or a subset).
#
#   ./run_tests.sh                        # all 48 configs, all tests
#   ./run_tests.sh --test utils_addr      # all 48 configs, one test target
#   BACKENDS="blake" SECPARS="128f" ./run_tests.sh
#
# Extra cargo args are passed through.
#
# Robustness note: the release cdylib is COPIED to a per-configuration path and
# the test harness is pointed at it via SPHINCS_RUST_SO. Without that, any other
# `cargo build` running concurrently overwrites target/release/libsphincs_core_det.so
# and the tests silently compare against a .so built for a different SECPAR.
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation" || exit 1
export CARGO_NET_OFFLINE=1
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/translation/target-test}"

SO_DIR="$ROOT/gen/so"
mkdir -p "$SO_DIR" "$ROOT/gen"

BACKENDS="${BACKENDS:-blake haraka sha2 shake}"
THASHES="${THASHES:-robust simple}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"
TEST_TIMEOUT="${TEST_TIMEOUT:-600}"

fail=0
for b in $BACKENDS; do
  for t in $THASHES; do
    for s in $SECPARS; do
      cfg="$b,$t,$s"
      tag="$b-$t-$s"
      if ! cargo build --release --no-default-features --features "$cfg" \
             > "$ROOT/gen/build-$tag.log" 2>&1; then
        echo "RUSTBUILD-FAIL $cfg"; fail=1; continue
      fi
      cp "$CARGO_TARGET_DIR/release/libsphincs_core_det.so" "$SO_DIR/lib-$tag.so" || {
        echo "COPY-FAIL $cfg"; fail=1; continue; }
      log="$ROOT/gen/test-$tag.log"
      if SPHINCS_RUST_SO="$SO_DIR/lib-$tag.so" \
         timeout "$TEST_TIMEOUT" cargo test --no-default-features \
            --features "$cfg" "$@" > "$log" 2>&1; then
        n=$(grep -c '^test .* ok$' "$log")
        echo "PASS $cfg ($n tests)"
      else
        echo "FAIL $cfg  -> $log"
        grep -E '^(test .* FAILED|error)' "$log" | head -5
        fail=1
      fi
    done
  done
done
if [ "$fail" = 0 ]; then echo "ALL CONFIGS PASSED"; else echo "SOME CONFIGS FAILED"; fi
exit $fail
