#!/usr/bin/env bash
# Differential test runner.
#
# `cargo test` does not relink a cdylib-only library, so the cdylib must be
# built explicitly first (same profile) or the tests would load a stale .so.
#
# Usage: ./run_tests.sh [extra cargo args...]
set -euo pipefail
cd "$(dirname "$0")"

ROOT="$(cd .. && pwd)"

# 1. Build the C reference shared object.
if [ ! -d "$ROOT/c_src/build" ] || [ -z "$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null)" ]; then
  echo "== building C shared library =="
  mkdir -p "$ROOT/c_src/build"
  (cd "$ROOT/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
fi

# 2. Enumerate feature combinations. This crate declares no [features], so the
#    only configuration is the default one; the loop keeps the workflow correct
#    if features are ever added.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)

COMBOS=("")
if [ -n "$FEATURES" ]; then
  # shellcheck disable=SC2206
  FEAT_ARR=($FEATURES)
  n=${#FEAT_ARR[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo="$combo${combo:+,}${FEAT_ARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  echo "===== feature combination: $label ====="
  timeout 600 cargo check --no-default-features --features "$combo" 2>&1 | tail -3
  timeout 600 cargo build --no-default-features --features "$combo" 2>&1 | tail -3
  timeout 600 cargo test --no-default-features --features "$combo" "$@" -- --test-threads=1 2>&1 | tail -20

  echo "-- symbol comparison (C vs Rust) --"
  c_so=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
  diff <(nm -D --defined-only "$c_so" | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only target/debug/libenvy_lib.so | awk '{print $NF}' | sort -u) \
    && echo "symbols identical"
done
echo "ALL FEATURE COMBINATIONS PASSED"
