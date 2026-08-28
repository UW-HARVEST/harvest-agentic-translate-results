#!/usr/bin/env bash
# Full differential verification across every profile and feature combination.
#
# IMPORTANT: `cargo test` does NOT rebuild a `cdylib` target -- for
# `crate-type = ["cdylib"]` it compiles `src/lib.rs` into a test-harness binary
# and leaves `target/<profile>/libdoubleneg_lib.so` alone. The explicit
# `cargo build` below is what makes the tests see current code. (The tests also
# refuse to run against a `.so` older than `src/`, as a backstop.)
set -euo pipefail

cd "$(dirname "$0")"
CARGO_FLAGS="--offline"

# --- 1. C reference library -------------------------------------------------
echo "=== building the C reference library ==="
(
  cd ../c_src
  mkdir -p build
  cd build
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
)
C_SO=$(ls ../c_src/build/lib*.so)
echo "C .so: $C_SO"

# --- 2. Feature combinations ------------------------------------------------
# Extracted from Cargo.toml. This crate declares no [features] table, so the
# only combinations are the default one and the explicitly-empty one; both are
# run so the loop stays correct if features are ever added.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "=== Cargo.toml declares no [features]; combinations: default, --no-default-features ==="
  COMBOS=("" "--no-default-features")
else
  echo "=== features found: $FEATURES ==="
  COMBOS=("" "--no-default-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
    COMBOS+=("--all-features")
  done
fi

FAILED=0
for PROFILE in release debug; do
  if [ "$PROFILE" = "release" ]; then PROFILE_FLAG="--release"; else PROFILE_FLAG=""; fi

  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features=[${COMBO:-default}]"
    echo
    echo "############################################################"
    echo "### $LABEL"
    echo "############################################################"

    # Build the cdylib FIRST -- `cargo test` will not do it.
    # shellcheck disable=SC2086
    cargo build $CARGO_FLAGS $PROFILE_FLAG $COMBO
    # shellcheck disable=SC2086
    if cargo test $CARGO_FLAGS $PROFILE_FLAG $COMBO -- --test-threads=1; then
      echo "### PASS: $LABEL"
    else
      echo "### FAIL: $LABEL"
      FAILED=1
    fi
  done
done

echo
if [ "$FAILED" -eq 0 ]; then
  echo "=========== ALL COMBINATIONS PASSED ==========="
else
  echo "=========== SOME COMBINATIONS FAILED ==========="
  exit 1
fi
