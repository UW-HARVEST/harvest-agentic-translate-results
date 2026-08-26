#!/bin/bash
# Full differential verification: builds the C .so and the Rust .so, then runs
# the Phase B/C/D differential tests for every feature combination.
#
# `cargo test` on its own does NOT rebuild a cdylib-only lib target (the tests
# dlopen the library rather than linking it), so the explicit `cargo build`
# before each `cargo test` is load-bearing, not decorative. tests/common/mod.rs
# additionally refuses to run against a stale artifact.
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"
rc=0

echo "=== building C shared library ==="
mkdir -p c_src/build
( cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }

# Cargo.toml has no [features] table and CMakeLists.txt has no options, so the
# complete set of build configurations is the single empty one. Enumerated here
# rather than hardcoded so the loop stays correct if features are ever added.
FEATURES=$(sed -n '/^\[features\]/,/^\[/p' Cargo.toml \
             | grep -oE '^[a-zA-Z0-9_-]+' | grep -v '^default$' || true)
if [ -z "$FEATURES" ]; then
  COMBOS=("")            # no features exist -> one configuration
else
  COMBOS=("")            # (would be the powerset of $FEATURES)
  for f in $FEATURES; do COMBOS+=("$f"); done
fi

for combo in "${COMBOS[@]}"; do
  label=${combo:-"<none>"}
  echo
  echo "=== configuration: --no-default-features --features '$label' ==="

  echo "--- cargo check ---"
  timeout 600 cargo check $CARGO_FLAGS --no-default-features --features "$combo" 2>&1 \
    | tail -5 || { echo "CHECK FAILED ($label)"; rc=1; continue; }

  # Rebuild the cdylib so the tests can never load a stale artifact.
  echo "--- cargo build (refresh cdylib) ---"
  timeout 600 cargo build $CARGO_FLAGS --no-default-features --features "$combo" 2>&1 \
    | tail -5 || { echo "BUILD FAILED ($label)"; rc=1; continue; }

  echo "--- cargo test ---"
  if timeout 600 cargo test $CARGO_FLAGS --no-default-features --features "$combo" 2>&1 \
       | tail -25; then
    echo "PASS ($label)"
  else
    echo "TEST FAILED ($label)"; rc=1
  fi
done

echo
if [ "$rc" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$rc"
