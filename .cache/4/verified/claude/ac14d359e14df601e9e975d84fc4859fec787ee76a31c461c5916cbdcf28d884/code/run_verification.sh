#!/usr/bin/env bash
# Full verification driver: builds the C reference .so, then for EVERY Cargo
# feature combination rebuilds the Rust cdylib, diffs the exported symbols and
# runs the Phase B + Phase C differential test suites.
#
# Usage: ./run_verification.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT=$(pwd)
FAILED=0

echo "==================================================================="
echo "Phase 0 — build the C reference shared library"
echo "==================================================================="
mkdir -p c_src/build
( cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C .so: $C_SO"

# -------------------------------------------------------------------------
# Enumerate every valid feature combination from Cargo.toml.
# -------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

COMBOS=("")   # the no-feature (== default, since no features exist) build
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo
echo "Declared features        : ${FEATURES[*]:-<none>}"
echo "Combinations to verify   : ${#COMBOS[@]}"

for combo in "${COMBOS[@]}"; do
  label=${combo:-"<no features>"}
  echo
  echo "==================================================================="
  echo "Feature combination: $label"
  echo "==================================================================="

  ARGS=(--no-default-features)
  [ -n "$combo" ] && ARGS+=(--features "$combo")

  echo "--- cargo check ---"
  if ! timeout 600 cargo check "${ARGS[@]}" 2>&1 | tail -5; then
    echo "!! cargo check FAILED for [$label]"; FAILED=1; continue
  fi

  echo "--- cargo build (cdylib) ---"
  if ! timeout 600 cargo build "${ARGS[@]}" 2>&1 | tail -5; then
    echo "!! cargo build FAILED for [$label]"; FAILED=1; continue
  fi

  RUST_SO="$ROOT/target/debug/libcollided_lib.so"

  echo "--- symbol parity (Phase D) ---"
  MISSING=$(comm -23 \
      <(nm -D --defined-only "$C_SO"    | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u))
  if [ -n "$MISSING" ]; then
    echo "!! symbols exported by C but MISSING from Rust:"; echo "$MISSING"; FAILED=1
  else
    echo "OK: every C symbol is exported by the Rust .so ($(nm -D --defined-only "$C_SO" | wc -l) symbols)"
  fi

  echo "--- differential tests (Phase B + C) ---"
  if timeout 600 cargo test "${ARGS[@]}" -- --test-threads=4 2>&1 | tail -40; then
    echo "OK: tests passed for [$label]"
  else
    echo "!! TESTS FAILED for [$label]"; FAILED=1
  fi
done

echo
echo "==================================================================="
if [ "$FAILED" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS VERIFIED"
else
  echo "VERIFICATION FAILED"
fi
echo "==================================================================="
exit "$FAILED"
