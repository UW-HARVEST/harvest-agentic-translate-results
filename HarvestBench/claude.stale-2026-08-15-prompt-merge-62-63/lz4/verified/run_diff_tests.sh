#!/usr/bin/env bash
# Build the C .so and the Rust cdylib, then run the differential tests for
# EVERY feature combination declared in Cargo.toml.
#
# Usage: ./run_diff_tests.sh [extra cargo test args...]
set -uo pipefail
cd "$(dirname "$0")"

# ---- 1. Build the C reference shared library -------------------------------
if [ ! -f c_src/build/liblz4.so ]; then
  echo "=== building C liblz4.so ==="
  ( mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

# ---- 2. Enumerate feature combinations ------------------------------------
# Every subset of the [features] table (excluding "default"), plus the
# no-default-features baseline. With no [features] section this yields the
# single default configuration.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("")            # only the default configuration exists
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "=== feature combinations to verify: ${#COMBOS[@]} ==="
for c in "${COMBOS[@]}"; do echo "  - '${c:-<default/none>}'"; done

# ---- 3. cargo check + build + test each combination ------------------------
rc=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/none>}"
  echo
  echo "############################################################"
  echo "### FEATURES: $label"
  echo "############################################################"

  if [ -n "$combo" ]; then
    FLAGS=(--no-default-features --features "$combo")
  else
    FLAGS=(--no-default-features)
  fi

  echo "--- cargo check ${FLAGS[*]} ---"
  if ! timeout 600 cargo check "${FLAGS[@]}" 2>&1 | grep -E "^(error|warning: unused)" | head -20; then :; fi
  if ! timeout 600 cargo check "${FLAGS[@]}" >/dev/null 2>&1; then
    echo "!!! cargo check FAILED for '$label'"; rc=1; continue
  fi

  # The cdylib must exist on disk before the tests dlopen it.
  echo "--- cargo build (cdylib) ---"
  if ! timeout 600 cargo build "${FLAGS[@]}" >/dev/null 2>&1; then
    echo "!!! cargo build FAILED for '$label'"; rc=1; continue
  fi
  ls -l target/debug/liblz4.so || { echo "!!! cdylib missing"; rc=1; continue; }

  echo "--- cargo test ---"
  timeout 600 cargo test "${FLAGS[@]}" "$@" 2>&1 | tail -60
  if [ "${PIPESTATUS[0]}" -ne 0 ]; then
    echo "!!! cargo test FAILED for '$label'"; rc=1
  fi
done

echo
if [ "$rc" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit "$rc"
