#!/usr/bin/env bash
# Phase D — run every phase across EVERY feature combination and both build
# profiles of the Rust .so.  Feature combinations are extracted from Cargo.toml
# rather than hard-coded.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

# --- enumerate feature combinations -----------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{sub(/[[:space:]]*=.*/,"");print}' Cargo.toml
)
echo "features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]:-none})"

COMBOS=("")                         # default features (no extra flags)
COMBOS+=("--no-default-features")   # nothing enabled
if [[ ${#FEATURES[@]} -gt 0 ]]; then
  # every non-empty subset (powerset) of the declared features
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do (( mask & (1<<i) )) && sel+=("${FEATURES[i]}"); done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

rc=0
for combo in "${COMBOS[@]}"; do
  flags="$combo"
  label="${combo:-<default features>}"
  echo
  echo "############ COMBO: ${label} ############"

  echo "---- cargo check ----"
  # shellcheck disable=SC2086
  timeout 300 cargo check $flags >/dev/null 2>&1 || { echo "FAIL cargo check ($label)"; rc=1; continue; }

  for profile in release debug; do
    echo "---- build .so ($profile) ----"
    if [[ $profile == release ]]; then
      # shellcheck disable=SC2086
      timeout 300 cargo build --release $flags >/dev/null 2>&1 || { echo "FAIL build ($label/$profile)"; rc=1; continue; }
      rm -f target/debug/libhdr_compare_lib.so   # force the harness to pick release
    else
      # shellcheck disable=SC2086
      timeout 300 cargo build $flags >/dev/null 2>&1 || { echo "FAIL build ($label/$profile)"; rc=1; continue; }
    fi

    echo "---- symbol parity ----"
    ./check_symbols.sh | tail -n 3 || rc=1

    echo "---- tests against the $profile .so ----"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test $flags -- --test-threads=2 2>&1 | grep -E '^test result|FAILED|panicked'; then
      echo "FAIL tests ($label/$profile)"; rc=1
    fi
  done
done

echo
if [[ $rc -eq 0 ]]; then echo "ALL COMBOS PASSED"; else echo "SOME COMBOS FAILED"; fi
exit $rc
