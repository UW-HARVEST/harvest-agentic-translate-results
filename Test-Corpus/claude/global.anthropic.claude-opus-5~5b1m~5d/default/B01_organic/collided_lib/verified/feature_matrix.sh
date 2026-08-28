#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# The combinations are extracted from Cargo.toml rather than hard-coded, so this
# keeps working if features are added later. This crate currently declares no
# [features] section, so the matrix is:
#     default            (== no features at all)
#     --no-default-features
# and both are still run explicitly for both profiles.
set -uo pipefail
cd "$(dirname "$0")"

# --- enumerate features from Cargo.toml -------------------------------------
features=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

combos=("" "--no-default-features")
if [ -n "${features}" ]; then
  echo "features declared: ${features}"
  # every non-empty subset of the declared features, with default off
  feat_arr=(${features})
  n=${#feat_arr[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    set=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then set="${set:+${set},}${feat_arr[$i]}"; fi
    done
    combos+=("--no-default-features --features ${set}")
    combos+=("--features ${set}")
  done
else
  echo "features declared: (none) — the default feature set is the only configuration"
fi

status=0
for profile in "" "--release"; do
  for combo in "${combos[@]}"; do
    label="profile=${profile:-dev} combo=${combo:-default}"
    echo
    echo "################ ${label}"
    # The cdylib MUST be rebuilt for each combo: `cargo test` never rebuilds a
    # cdylib-only lib, so without this the tests would dlopen a stale .so.
    if ! timeout 600 cargo build ${profile} ${combo} 2>&1 | tail -1; then
      echo "FAIL(build): ${label}"; status=1; continue
    fi
    out=$(timeout 600 cargo test ${profile} ${combo} 2>&1)
    echo "${out}" | grep -E "^(running|test result)"
    if echo "${out}" | grep -qE "^test result: FAILED|error\[|error:"; then
      echo "FAIL(test): ${label}"
      echo "${out}" | grep -E "^(test .*FAILED|error)" | head -20
      status=1
    fi
  done
done

echo
if [ ${status} -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASS"; else echo "SOME COMBINATIONS FAILED"; fi
exit ${status}
