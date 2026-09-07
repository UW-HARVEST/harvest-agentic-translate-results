#!/usr/bin/env bash
# Differential-test driver.
#
#   1. (re)builds the C shared library
#   2. (re)builds the Rust cdylib   -- `cargo test` alone does NOT rebuild a
#      cdylib, so this step is mandatory or the tests would dlopen a stale .so
#   3. runs the requested test targets under every feature combination
#
# Usage: ./run_diff_tests.sh [extra cargo-test args...]
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"

export CARGO_NET_OFFLINE=true
CARGO_FLAGS=(--offline)

echo "==> building C shared library"
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build"
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
)
ls -1 "$ROOT"/c_src/build/*.so

# Enumerate feature combinations from Cargo.toml. This crate declares no
# [features] table, so the list collapses to the single default configuration;
# the loop is written generically so new features are picked up automatically.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[a-zA-Z0-9_-]+[[:space:]]*=/{print $1}' \
    "$HERE/Cargo.toml"
)

COMBOS=("default")
if ((${#FEATURES[@]} > 0)); then
  COMBOS=("default" "none")
  for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
  # all features together
  COMBOS+=("$(
    IFS=,
    echo "${FEATURES[*]}"
  )")
fi

status=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
  default) FEATFLAGS=() ;;
  none) FEATFLAGS=(--no-default-features) ;;
  *) FEATFLAGS=(--no-default-features --features "$combo") ;;
  esac

  echo
  echo "############################################################"
  echo "### feature combination: $combo"
  echo "############################################################"

  # --- debug cdylib -------------------------------------------------------
  ( cd "$HERE" && cargo build "${CARGO_FLAGS[@]}" "${FEATFLAGS[@]}" )
  ( cd "$HERE" && cargo test  "${CARGO_FLAGS[@]}" "${FEATFLAGS[@]}" "$@" ) || status=1

  # --- release cdylib (the shipping artifact; optimization can reassociate
  #     float ops, so it must be verified separately) ------------------------
  echo "--- re-running against the RELEASE cdylib ---"
  ( cd "$HERE" && cargo build "${CARGO_FLAGS[@]}" "${FEATFLAGS[@]}" --release )
  (
    cd "$HERE"
    AGGLOM_RUST_SO="$HERE/target/release/libagglom_lib.so" \
      cargo test "${CARGO_FLAGS[@]}" "${FEATFLAGS[@]}" --release "$@"
  ) || status=1
done

exit "$status"
