#!/usr/bin/env bash
# Runs the differential suite across every cargo feature combination and both
# build profiles. `Cargo.toml` declares no [features], so the combination set is
# the single empty combo; the loop is written generically so it stays correct if
# features are ever added.
set -uo pipefail
cd "$(dirname "$0")"

# --- ensure the C reference library exists -----------------------------------
if [ ! -f ../c_src/build/libdriver.so ]; then
  echo "building C reference library"
  (cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || exit 1
fi

# --- enumerate feature combinations from Cargo.toml ---------------------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=("")                       # no-default-features, nothing enabled
if [ -n "$FEATURES" ]; then
  for f in $FEATURES; do COMBOS+=("$f"); done
  COMBOS+=("$(echo "$FEATURES" | tr '\n' ',' | sed 's/,$//')")   # all at once
fi
echo "feature combinations: ${#COMBOS[@]}"

FAIL=0
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then FLAGS=(--no-default-features)
  else                     FLAGS=(--no-default-features --features "$combo"); fi
  label="${combo:-<none>}"

  # Build BOTH cdylib profiles so the harness compares C against debug
  # (panic=unwind) and release (panic=abort) Rust artifacts.
  timeout 600 cargo build          "${FLAGS[@]}" >/dev/null 2>&1 || { echo "BUILD FAIL debug   [$label]";   FAIL=1; continue; }
  timeout 600 cargo build --release "${FLAGS[@]}" >/dev/null 2>&1 || { echo "BUILD FAIL release [$label]"; FAIL=1; continue; }

  for prof in "" "--release"; do
    echo "=============================================================="
    echo ">>> features=[$label] test-profile=${prof:-debug}"
    if ! timeout 600 cargo test $prof "${FLAGS[@]}" --test difftest 2>&1 | tail -60; then
      FAIL=1
    fi
  done
done

echo "=============================================================="
if [ "$FAIL" -ne 0 ]; then echo "OVERALL: FAILURES PRESENT"; exit 1; fi
echo "OVERALL: ALL FEATURE COMBINATIONS PASSED"
