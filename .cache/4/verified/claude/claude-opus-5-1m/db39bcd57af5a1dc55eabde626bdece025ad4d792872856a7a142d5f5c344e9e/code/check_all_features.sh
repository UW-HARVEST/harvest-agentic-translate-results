#!/usr/bin/env bash
# Enumerate EVERY valid feature combination declared in Cargo.toml and run
# `cargo check` for each one.  Also used by run_all_tests.sh.
#
# Usage: ./check_all_features.sh [cargo-subcommand ...]   (default: check)
set -uo pipefail

cd "$(dirname "$0")"

CARGO_CMD=("${@:-check}")

# ---- Extract the [features] table from Cargo.toml -------------------------
# Print every key in the [features] section, skipping the implicit "default".
mapfile -t FEATURES < <(
  awk '
    /^[[:space:]]*\[/ { in_f = ($0 ~ /^[[:space:]]*\[features\][[:space:]]*$/); next }
    in_f && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); gsub(/[[:space:]]/, "");
      if ($0 != "default") print
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "Declared non-default features in Cargo.toml: $N ${FEATURES[*]:-(none)}"

# ---- Build the list of combinations (power set of FEATURES) ---------------
COMBOS=()
if [ "$N" -eq 0 ]; then
  # No [features] table at all => the ONLY valid configuration is the empty set.
  COMBOS+=("")
else
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "Total feature combinations to verify: ${#COMBOS[@]}"
echo

rc=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<empty: --no-default-features>}"
  echo "=============================================================="
  echo ">>> cargo ${CARGO_CMD[*]} --no-default-features --features '$combo'"
  echo "=============================================================="
  # An empty combo must NOT emit a dangling `--features` (cargo rejects it).
  if [ -z "$combo" ]; then
    fargs=(--no-default-features)
  else
    fargs=(--no-default-features --features "$combo")
  fi
  if cargo "${CARGO_CMD[@]}" --offline "${fargs[@]}" 2>&1 | tail -n 30; then
    echo "RESULT: PASS  [$label]"
  else
    echo "RESULT: FAIL  [$label]"
    rc=1
  fi
  echo
done

# Also verify the default configuration (features enabled by `default`).
echo "=============================================================="
echo ">>> cargo ${CARGO_CMD[*]} (default features)"
echo "=============================================================="
if cargo "${CARGO_CMD[@]}" --offline 2>&1 | tail -n 30; then
  echo "RESULT: PASS  [default]"
else
  echo "RESULT: FAIL  [default]"
  rc=1
fi
echo

echo "=============================================================="
echo ">>> cargo ${CARGO_CMD[*]} --all-features"
echo "=============================================================="
if cargo "${CARGO_CMD[@]}" --offline --all-features 2>&1 | tail -n 30; then
  echo "RESULT: PASS  [all-features]"
else
  echo "RESULT: FAIL  [all-features]"
  rc=1
fi

echo
if [ "$rc" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (cargo ${CARGO_CMD[*]})"
else
  echo "SOME FEATURE COMBINATIONS FAILED (cargo ${CARGO_CMD[*]})"
fi
exit "$rc"
