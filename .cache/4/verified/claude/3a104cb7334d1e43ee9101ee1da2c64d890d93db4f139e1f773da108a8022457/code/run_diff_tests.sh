#!/usr/bin/env bash
# Differential C-vs-Rust verification driver.
#
# `cargo test` does not rebuild a `crate-type = ["cdylib"]` library, so the
# cdylib MUST be built explicitly before the integration tests run (the tests
# also refuse to run against a stale artifact — see tests/common/mod.rs).
#
# Iterates over every valid feature combination x every Cargo profile.
set -euo pipefail

cd "$(dirname "$0")"
ROOT=$PWD

CARGO_FLAGS=(--offline)

# --- 1. build the C shared library -----------------------------------------
mkdir -p c_src/build
(cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
C_SO=$ROOT/c_src/build/libtranslated_rust.so
test -f "$C_SO" || { echo "FATAL: $C_SO not built"; exit 1; }
echo "C library:    $C_SO"

# --- 2. enumerate feature combinations -------------------------------------
# Cargo.toml has no [features] section, so the only valid combination is the
# empty one. Derived mechanically so a future [features] block is picked up.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{print $1}' Cargo.toml
)
COMBOS=("")
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "feature combos: ${#COMBOS[@]} -> [$(printf '"%s" ' "${COMBOS[@]}")]"

# --- 3. cargo check every combination --------------------------------------
for combo in "${COMBOS[@]}"; do
  echo "== cargo check --no-default-features --features '$combo'"
  cargo check "${CARGO_FLAGS[@]}" --no-default-features --features "$combo" --all-targets
done

# --- 4. build + test every combination in both profiles --------------------
FAIL=0
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    prof_flag=(); prof_dir=debug
    if [ "$profile" = release ]; then prof_flag=(--release); prof_dir=release; fi

    echo
    echo "======================================================================"
    echo "== features='$combo'  profile=$profile"
    echo "======================================================================"
    cargo build "${CARGO_FLAGS[@]}" --no-default-features --features "$combo" "${prof_flag[@]}"
    RUST_SO=$ROOT/target/$prof_dir/libcircle_collide_lib.so
    test -f "$RUST_SO" || { echo "FATAL: $RUST_SO not built"; exit 1; }

    if ! C_LIB_PATH="$C_SO" RUST_LIB_PATH="$RUST_SO" \
         cargo test "${CARGO_FLAGS[@]}" --no-default-features --features "$combo" \
         "${prof_flag[@]}" -- --test-threads="$(nproc)"; then
      echo "FAILED: features='$combo' profile=$profile"
      FAIL=1
    fi
  done
done

echo
if [ "$FAIL" = 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; exit 1; fi
