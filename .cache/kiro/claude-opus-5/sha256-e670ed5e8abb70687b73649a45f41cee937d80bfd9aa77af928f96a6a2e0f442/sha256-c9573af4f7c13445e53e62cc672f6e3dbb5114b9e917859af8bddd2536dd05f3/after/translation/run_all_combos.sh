#!/usr/bin/env bash
# Phase D: run the differential suite across every feature combination and both
# cargo profiles. Feature list is extracted from Cargo.toml, not hardcoded.
set -uo pipefail

cd "$(dirname "$0")"

# --- enumerate declared features -------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,""); gsub(/ /,""); if ($0 != "default" && $0 != "") print}' Cargo.toml
)
echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the feature-combination matrix (powerset) ------------------------
COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  COMBOS+=("")                        # default (only) configuration
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    set=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then set="$set,${FEATURES[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${set#,}")
  done
  COMBOS+=("")            # default features
  COMBOS+=("--all-features")
fi

fail=0
for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo test ${profile_flag:-<debug>} ${combo:-<default-features>}"
    echo "==================================================================="
    echo ">>> $label"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo check $profile_flag $combo >/tmp/pd-check.log 2>&1; then
      echo "!!! cargo check FAILED for $label"; tail -20 /tmp/pd-check.log; fail=1; continue
    fi
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $profile_flag $combo >/tmp/pd-build.log 2>&1; then
      echo "!!! cargo build FAILED for $label"; tail -20 /tmp/pd-build.log; fail=1; continue
    fi
    # shellcheck disable=SC2086
    if timeout 600 cargo test $profile_flag $combo >/tmp/pd-test.log 2>&1; then
      grep -E '^result:' /tmp/pd-test.log | sed 's/^/    /'
    else
      echo "!!! TESTS FAILED for $label"
      grep -E '^(result:|failures:|    [a-z])' /tmp/pd-test.log | tail -40
      fail=1
    fi
  done
done

echo "==================================================================="
if [ "$fail" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS x PROFILES PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit "$fail"
