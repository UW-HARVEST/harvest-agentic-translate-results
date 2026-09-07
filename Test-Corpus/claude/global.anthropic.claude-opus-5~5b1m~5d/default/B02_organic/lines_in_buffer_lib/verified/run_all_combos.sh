#!/bin/sh
# Enumerate every Cargo feature combination and run the full differential
# test-suite under each, in both the debug (overflow-checks=ON) and release
# (overflow-checks=OFF) profiles.
#
# `cargo test` does NOT build the cdylib artifact, so the library is built
# explicitly for each profile/feature set *before* the tests run, otherwise the
# harness would have nothing (or the wrong profile's .so) to dlopen.
set -e
cd "$(dirname "$0")"

# Make sure the C reference library exists.
if [ ! -f ../c_src/build/libdriver.so ]; then
  ( mkdir -p ../c_src/build && cd ../c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null )
fi

FEATURES=$(awk '/^\[features\]/{f=1;next}/^\[/{f=0}f&&/=/{sub(/ *=.*/,"");print}' Cargo.toml \
           | grep -v '^default$' || true)
echo "declared cargo features: [${FEATURES:-<none>}]"

fail=0
run() {
  echo "=== combo: cargo test --offline $* ==="
  cargo build --offline "$@" >/dev/null 2>&1 || { echo "BUILD FAILED"; fail=1; return; }
  # Run the suite exactly once and judge it by cargo's exit status, so a flaky
  # pass on a retry can never mask a failure.
  out=$(cargo test --offline "$@" 2>&1); rc=$?
  printf '%s\n' "$out" | grep -E 'test result|FAILED|^error' || true
  [ "$rc" -eq 0 ] || { echo ">>> COMBO FAILED: $*"; fail=1; }
}

run                                   # default features, debug
run --release                         # default features, release
run --no-default-features             # empty feature set, debug
run --release --no-default-features   # empty feature set, release

for f in $FEATURES; do
  run --no-default-features --features "$f"
  run --release --no-default-features --features "$f"
done
if [ -n "$FEATURES" ]; then
  run --all-features
  run --release --all-features
fi

[ "$fail" -eq 0 ] && echo "ALL COMBOS PASSED" || { echo "SOME COMBOS FAILED"; exit 1; }
