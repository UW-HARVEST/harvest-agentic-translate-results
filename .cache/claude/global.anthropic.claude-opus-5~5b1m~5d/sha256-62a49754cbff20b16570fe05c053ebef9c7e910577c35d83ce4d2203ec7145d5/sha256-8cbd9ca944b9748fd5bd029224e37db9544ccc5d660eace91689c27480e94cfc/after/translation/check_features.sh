#!/usr/bin/env bash
# Phase D — enumerate every feature combination from Cargo.toml and run the full
# differential suite under each one.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

# ---- enumerate the declared features --------------------------------------
# (everything under [features] except the `default` meta-feature)
features=$(awk '
  /^\[features\]/       { inf=1; next }
  /^\[/                 { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }
' Cargo.toml || true)

n=$(printf '%s' "$features" | grep -c . || true)
echo "declared features: ${n}"

if [ "$n" -eq 0 ]; then
  echo "Cargo.toml declares no [features] table."
  echo "=> the only configuration is the default one; --no-default-features is equivalent."
  combos=("" "--no-default-features")
else
  # full power set of the declared features, plus the default build
  mapfile -t flist <<<"$features"
  combos=("")
  total=$((1 << ${#flist[@]}))
  for ((m = 0; m < total; m++)); do
    sel=()
    for ((i = 0; i < ${#flist[@]}; i++)); do
      (((m >> i) & 1)) && sel+=("${flist[$i]}")
    done
    if [ ${#sel[@]} -eq 0 ]; then
      combos+=("--no-default-features")
    else
      combos+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
fi

# ---- run the suite under each combination ---------------------------------
fail=0
for combo in "${combos[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "==================================================================="
  echo "FEATURE COMBO: $label"
  echo "==================================================================="
  if CARGO_FEATURE_ARGS="$combo" ./run_tests.sh 2>&1 | tee "${TMPDIR:-/tmp}/combo.$$" | grep -E '^test result'; then
    if grep -q 'FAILED' "${TMPDIR:-/tmp}/combo.$$"; then
      echo "!!! FAILED under: $label"
      fail=1
    fi
  else
    echo "!!! ERROR under: $label"
    fail=1
  fi
  rm -f "${TMPDIR:-/tmp}/combo.$$"
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#combos[@]} combos)"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
  exit 1
fi
