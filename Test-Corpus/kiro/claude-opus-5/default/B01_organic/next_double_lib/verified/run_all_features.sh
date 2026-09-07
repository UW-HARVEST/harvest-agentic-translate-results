#!/usr/bin/env bash
# Phase D: run the differential suite under EVERY feature combination.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# this stays correct if features are ever added.
set -uo pipefail
cd "$(dirname "$0")"

# --- enumerate features -----------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {gsub(/[[:space:]]*=.*/,""); print}
' Cargo.toml | grep -v '^default$' | sort -u)

COMBOS=("--all-features" "--no-default-features")
if [ -n "$FEATURES" ]; then
  # every non-empty subset of the declared features
  feats=($FEATURES)
  n=${#feats[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${feats[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
  done
else
  echo "note: Cargo.toml declares no [features]; the only build configurations"
  echo "      are default == --all-features == --no-default-features."
fi
COMBOS+=("")   # default features

FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  echo "=============================================================="
  echo "== $label"
  echo "=============================================================="

  # Both .so profiles must exist: the suite loads and compares against each.
  timeout 600 cargo build          $combo >/dev/null 2>&1 || { echo "BUILD(debug) FAILED: $label";   FAIL=1; continue; }
  timeout 600 cargo build --release $combo >/dev/null 2>&1 || { echo "BUILD(release) FAILED: $label"; FAIL=1; continue; }
  timeout 600 cargo check --all-targets $combo >/dev/null 2>&1 || { echo "CHECK FAILED: $label"; FAIL=1; continue; }

  for profile in "" "--release"; do
    if timeout 600 cargo test $profile $combo 2>&1 | tail -n 3 | sed 's/^/    /'; then :; fi
    st=${PIPESTATUS[0]}
    if [ "$st" -ne 0 ]; then
      echo "TEST FAILED: $label ${profile:-<dev profile>}"
      FAIL=1
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit $FAIL
