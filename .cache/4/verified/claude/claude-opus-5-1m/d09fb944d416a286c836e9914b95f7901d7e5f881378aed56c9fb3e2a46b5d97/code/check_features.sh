#!/usr/bin/env bash
# Mechanically enumerate every feature combination declared in Cargo.toml
# (the power set of the [features] table, minus any implied-only entries) and
# run `cargo check` / `cargo test` for each one.
#
# Usage:  ./check_features.sh check     # cargo check every combo
#         ./check_features.sh test      # cargo test  every combo
set -uo pipefail

cd "$(dirname "$0")"
MODE="${1:-check}"

# --- extract feature names from the [features] section of Cargo.toml ---------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { in_f = 1; next }
    /^\[/                { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

N=${#FEATURES[@]}
echo "Declared (non-default) features: ${N} -> ${FEATURES[*]:-<none>}"

# --- build the power set ----------------------------------------------------
COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if ((mask & (1 << i))); then
      combo+="${combo:+,}${FEATURES[i]}"
    fi
  done
  COMBOS+=("$combo")
done

echo "Total combinations to verify: ${#COMBOS[@]}"
echo

FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  echo "=============================================================="
  echo "### cargo $MODE --no-default-features --features '$label'"
  echo "=============================================================="
  if [[ "$MODE" == "test" ]]; then
    timeout 600 cargo test --offline --no-fail-fast --no-default-features --features "$combo" -- --test-threads=1
  else
    timeout 600 cargo "$MODE" --offline --no-default-features --features "$combo" --all-targets
  fi
  rc=$?
  if ((rc != 0)); then
    echo ">>> FAILED (rc=$rc) for features: $label"
    FAIL=1
  else
    echo ">>> OK for features: $label"
  fi
  echo
done

# --- also verify the default feature set ------------------------------------
echo "=============================================================="
echo "### cargo $MODE (default features)"
echo "=============================================================="
if [[ "$MODE" == "test" ]]; then
  timeout 600 cargo test --offline --no-fail-fast -- --test-threads=1
else
  timeout 600 cargo "$MODE" --offline --all-targets
fi
rc=$?
((rc != 0)) && { echo ">>> FAILED (rc=$rc) for default features"; FAIL=1; } || echo ">>> OK for default features"

# --- and the release profile (`panic = "abort"` differs from dev) ------------
if [[ "$MODE" == "test" ]]; then
  echo "=============================================================="
  echo "### differential tests against the RELEASE-profile cdylib"
  echo "###   (Cargo.toml sets panic = \"abort\" for [profile.release])"
  echo "=============================================================="
  for combo in "${COMBOS[@]}"; do
    label="${combo:-<no features>}"
    echo "--- release cdylib, features: $label"
    timeout 600 cargo build --offline --release --lib --no-default-features \
      ${combo:+--features "$combo"} --target-dir target/ffi-so-release || { FAIL=1; continue; }
    so="$PWD/target/ffi-so-release/release/libdriver.so"
    if [[ ! -f "$so" ]]; then echo ">>> release cdylib missing at $so"; FAIL=1; continue; fi
    DRIVER_RUST_SO="$so" timeout 600 cargo test --offline --no-fail-fast --no-default-features \
      --features "$combo" -- --test-threads=1
    rc=$?
    ((rc != 0)) && { echo ">>> FAILED (rc=$rc) release/$label"; FAIL=1; } || echo ">>> OK release/$label"
  done
  echo
fi

echo
if ((FAIL == 0)); then echo "ALL FEATURE COMBINATIONS PASSED ($MODE)"; else echo "SOME FEATURE COMBINATIONS FAILED ($MODE)"; fi
exit $FAIL
