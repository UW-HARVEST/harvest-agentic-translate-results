#!/usr/bin/env bash
# Build the C ground-truth .so and the Rust .so, then run the differential suite.
#
# `cargo test` does NOT build a cdylib-only lib target (integration tests do not
# link against it — they dlopen it), so the cdylib must be built explicitly and
# with the SAME profile the tests run under, or the tests would load a stale .so.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
CARGO_FLAGS="--offline"

# ---- 1. C ground truth ---------------------------------------------------
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
echo "built: $root/c_src/build/libdriver.so"

# ---- 2. Rust .so (debug profile == the one `cargo test` uses) ------------
cd "$here"
cargo build $CARGO_FLAGS ${EXTRA_FEATURES:-}
echo "built: $here/target/debug/libdriver.so"

# ---- 3. Symbol parity (Phase A / D) -------------------------------------
echo
echo "=== symbol parity: nm -D ==="
diff <(nm -D --defined-only "$root/c_src/build/libdriver.so" | awk '{print $3}' | sort) \
     <(nm -D --defined-only "$here/target/debug/libdriver.so" | awk '{print $3}' | sort) \
  && echo "OK: symbol diff is empty"

# ---- 4. Differential tests ----------------------------------------------
echo
cargo test $CARGO_FLAGS ${EXTRA_FEATURES:-} -- --test-threads="${TEST_THREADS:-4}" "$@"
