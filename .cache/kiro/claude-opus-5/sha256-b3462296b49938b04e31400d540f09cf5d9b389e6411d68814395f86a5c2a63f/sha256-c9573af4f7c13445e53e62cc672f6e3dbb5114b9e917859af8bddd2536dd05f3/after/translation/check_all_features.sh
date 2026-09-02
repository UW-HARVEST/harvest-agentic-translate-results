#!/usr/bin/env bash
# Phase D — enumerate every feature combination declared in Cargo.toml and run
# `cargo check` + the full differential test suite for each.
#
# The feature list is extracted mechanically from Cargo.toml rather than
# hard-coded, so adding a feature automatically widens the matrix.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- extract feature names from the [features] section of Cargo.toml ----------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/            { in_f = 1; next }
    /^\[/                      { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the combination list ----------------------------------------------
# Always include: default features, and no-default-features.
COMBOS=()
COMBOS+=("--all-features")
COMBOS+=("")                      # default features
COMBOS+=("--no-default-features")

n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  # full power set of the declared features, with default features off
  for (( mask = 1; mask < (1 << n); mask++ )); do
    sel=()
    for (( i = 0; i < n; i++ )); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
elif (( n > 12 )); then
  echo "WARNING: $n features -> power set too large; testing each feature alone."
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
fi

# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

echo "combinations to verify: ${#COMBOS[@]}"
echo

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  echo "=============================================================="
  echo ">>> $label"
  echo "=============================================================="

  # shellcheck disable=SC2086
  if ! timeout 300 cargo check $combo >/tmp/fc_check.log 2>&1; then
    echo "CHECK FAILED: $label"; tail -30 /tmp/fc_check.log; fail=1; continue
  fi
  echo "cargo check: ok"

  # The tests load the RELEASE cdylib, so rebuild it for this combination.
  # shellcheck disable=SC2086
  if ! timeout 300 cargo build --release $combo >/tmp/fc_build.log 2>&1; then
    echo "RELEASE BUILD FAILED: $label"; tail -30 /tmp/fc_build.log; fail=1; continue
  fi
  echo "cargo build --release: ok"

  # shellcheck disable=SC2086
  if ! timeout 600 cargo test $combo >/tmp/fc_test.log 2>&1; then
    echo "TESTS FAILED: $label"
    grep -E '^test |test result|panicked' /tmp/fc_test.log | tail -40
    fail=1; continue
  fi
  grep -E 'test result' /tmp/fc_test.log
  echo "cargo test: ok"
  echo
done

echo "=============================================================="
if (( fail )); then
  echo "RESULT: at least one feature combination FAILED"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} feature combination(s) passed"
