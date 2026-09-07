#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination and
# both cargo profiles. Feature combinations are extracted from Cargo.toml rather
# than hard-coded, so a future [features] section is picked up automatically.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- extract feature names from the [features] section, if any -------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "features declared: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the combination list -------------------------------------------
COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] section: default and --no-default-features are the same
  # single configuration. Both spellings are still exercised.
  COMBOS+=("")
  COMBOS+=("--no-default-features")
else
  COMBOS+=("")
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel+=("${FEATURES[$i]}"); fi
    done
    joined=$(
      IFS=,
      echo "${sel[*]}"
    )
    COMBOS+=("--no-default-features --features $joined")
  done
  COMBOS+=("--all-features")
fi

FAIL=0
for profile_flag in "--release" ""; do
  prof_label=${profile_flag:-debug}
  for combo in "${COMBOS[@]}"; do
    label="profile=${prof_label} combo='${combo:-<default>}'"
    echo "=============================================================="
    echo ">>> $label"
    # The tests dlopen the cdylib from target/<profile>/, so build it first
    # under the exact same flags.
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $profile_flag $combo >/dev/null 2>&1; then
      echo "BUILD FAILED: $label"
      FAIL=1
      continue
    fi
    # shellcheck disable=SC2086
    if timeout 600 cargo test $profile_flag $combo 2>&1 | tail -n 40; then
      echo "PASS: $label"
    else
      echo "FAIL: $label"
      FAIL=1
    fi
  done
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$FAIL"
