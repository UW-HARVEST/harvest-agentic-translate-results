#!/usr/bin/env bash
# Phase A/D helper: enumerate EVERY valid feature combination declared in
# Cargo.toml and run `cargo check` for each one.
#
# Usage: ./check_features.sh
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS=${CARGO_FLAGS:---offline}

# --- Enumerate the feature names from the [features] table (if any) ----------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); print
    }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- Build the powerset of feature names ------------------------------------
COMBOS=("")
n=${#FEATURES[@]}
if (( n > 0 )); then
  total=$(( 1 << n ))
  COMBOS=()
  for (( mask = 0; mask < total; mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( (mask >> i) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "feature combinations to verify: ${#COMBOS[@]}"

rc=0
for combo in "${COMBOS[@]}"; do
  label=${combo:-"<none>"}
  echo "=== cargo check --no-default-features --features '$label' ==="
  if ! timeout 600 cargo check $CARGO_FLAGS --all-targets \
        --no-default-features --features "$combo"; then
    echo "FAILED: --no-default-features --features '$label'"
    rc=1
  fi
done

# The default feature set as well (identical to <none> when no `default` exists).
echo "=== cargo check (default features) ==="
if ! timeout 600 cargo check $CARGO_FLAGS --all-targets; then
  echo "FAILED: default features"
  rc=1
fi

if (( rc == 0 )); then
  echo "ALL FEATURE COMBINATIONS CHECK CLEAN"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $rc
