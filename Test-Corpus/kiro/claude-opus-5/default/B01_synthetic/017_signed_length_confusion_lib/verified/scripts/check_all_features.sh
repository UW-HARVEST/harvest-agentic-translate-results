#!/usr/bin/env bash
# Enumerates every valid feature combination declared in translation/Cargo.toml
# and runs `cargo check` plus `cargo test` for each. The crate currently
# declares no [features], so the sweep degenerates to the single default
# (feature-less) configuration -- the script still derives that from the
# manifest rather than assuming it.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1   # translation/

# Feature names declared in [features], excluding the "default" meta-feature.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# Every subset of FEATURES, as comma separated strings ("" = no features).
COMBOS=("")
for f in "${FEATURES[@]}"; do
  for existing in "${COMBOS[@]}"; do
    if [ -z "$existing" ]; then COMBOS+=("$f"); else COMBOS+=("$existing,$f"); fi
  done
done

# The default feature set is also a valid configuration in its own right.
COMBOS+=("__default__")

status=0
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "__default__" ]; then
    label="(default features)"
    args=()
  elif [ -z "$combo" ]; then
    label="(no features)"
    args=(--no-default-features)
  else
    label="$combo"
    args=(--no-default-features --features "$combo")
  fi

  echo "=============================================================="
  echo "### $label"
  echo "=============================================================="

  for step in check build test; do
    echo "--- cargo $step ${args[*]}"
    if ! timeout 600 cargo "$step" "${args[@]}" 2>&1 | tail -25; then
      echo "!!! cargo $step FAILED for $label"
      status=1
    fi
  done
done

echo "=============================================================="
if [ "$status" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} configurations)"
else
  echo "FAILURES PRESENT"
fi
exit "$status"
