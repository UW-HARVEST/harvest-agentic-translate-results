#!/usr/bin/env bash
# Full differential verification run.
#
# Builds the C .so and the Rust .so, then runs every test in both the debug and
# release profiles. `--test-threads=1` is required: stdout is captured at the
# file-descriptor level, which is process-global.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"

CARGO_FLAGS="--offline"

echo "=== building the C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . )
ls -l "$ROOT/c_src/build/libdriver.so"

cd "$HERE"

fail=0
for profile in "" "--release"; do
  label="${profile:-debug}"
  echo
  echo "=== building the Rust cdylib ($label) ==="
  cargo build $CARGO_FLAGS $profile
  echo "=== running the differential suite ($label) ==="
  if ! cargo test $CARGO_FLAGS $profile -- --test-threads=1; then
    echo "!!! FAILURES in profile $label"
    fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL DIFFERENTIAL TESTS PASSED (debug + release)"
else
  echo "SOME DIFFERENTIAL TESTS FAILED"
fi
exit "$fail"
