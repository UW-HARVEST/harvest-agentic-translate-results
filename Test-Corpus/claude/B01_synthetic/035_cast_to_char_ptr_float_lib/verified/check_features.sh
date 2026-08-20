#!/usr/bin/env bash
# Enumerate EVERY valid feature combination declared in Cargo.toml and run
# `cargo check` (and optionally `cargo test`) for each one.
#
# Usage: ./check_features.sh [check|test|build]
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CMD="${1:-check}"

# --- Mechanically extract the feature names from the [features] table --------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "non-default features declared: $N ${FEATURES[*]:-(none)}"

# --- Build the power set of the feature list (the full combination space) ----
COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if (((mask >> i) & 1)); then
      combo="${combo:+$combo,}${FEATURES[$i]}"
    fi
  done
  COMBOS+=("$combo")
done

echo "feature combinations to verify: ${#COMBOS[@]}"

rc=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<empty (no features)>}"
  echo "=============================================================="
  echo ">>> cargo $CMD --no-default-features --features '$label'"
  echo "=============================================================="
  if ! timeout 600 cargo "$CMD" --offline --no-default-features --features "$combo"; then
    echo "!!! FAILED for feature combination: $label"
    rc=1
  fi
done

# The default feature set is also a valid configuration; verify it too.
echo "=============================================================="
echo ">>> cargo $CMD (default features)"
echo "=============================================================="
if ! timeout 600 cargo "$CMD" --offline; then
  echo "!!! FAILED for default feature set"
  rc=1
fi

exit "$rc"
