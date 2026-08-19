#!/usr/bin/env bash
# Full verification run: symbol parity (Phase D) + the differential suites
# (Phases B and C) for every valid feature combination (Phase A).
set -uo pipefail
cd "$(dirname "$0")"

fail=0

echo "############################################################"
echo "# Phase A: feature-combination enumeration + cargo check"
echo "############################################################"
./check_all_features.sh || fail=1

# Enumerate the feature combinations again for the test loop.
mapfile -t FEATS < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[ ]*=/ {
    split($0, a, "="); gsub(/[ \t]/, "", a[1]);
    if (a[1] != "default") print a[1];
  }' Cargo.toml)
n=${#FEATS[@]}
total=$(( 1 << n ))

for (( mask=0; mask<total; mask++ )); do
  combo=()
  for (( b=0; b<n; b++ )); do
    if (( (mask >> b) & 1 )); then combo+=("${FEATS[$b]}"); fi
  done
  joined=$(IFS=,; echo "${combo[*]}")
  label=${joined:-"<none>"}
  if [ -n "$joined" ]; then
    args="--no-default-features --features $joined"
  else
    args="--no-default-features"
  fi

  echo
  echo "############################################################"
  echo "# feature combination: $label"
  echo "############################################################"

  echo "--- Phase D: symbol parity ---"
  CARGO_EXTRA="$args" ./check_symbols.sh || fail=1

  echo "--- Phases B and C: differential tests ---"
  # The harness rebuilds the cdylib itself; tell it which features to use.
  DIFFTEST_CARGO_ARGS="$args" cargo test --offline $args 2>&1 | tail -80
  # shellcheck disable=SC2181
  if [ "${PIPESTATUS[0]}" != "0" ]; then fail=1; fi
done

echo
echo "############################################################"
if [ "$fail" = "0" ]; then
  echo "# ALL CHECKS PASSED"
else
  echo "# FAILURES DETECTED"
fi
echo "############################################################"
exit $fail
