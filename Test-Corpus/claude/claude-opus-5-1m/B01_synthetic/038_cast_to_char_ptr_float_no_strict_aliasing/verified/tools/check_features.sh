#!/bin/bash
# Phase A/D -- mechanically enumerate every valid feature combination from
# Cargo.toml and run `cargo check` (and optionally `cargo test`) for each.
#
# Usage: tools/check_features.sh [check|test]
set -u
cd "$(dirname "$0")/.."

MODE=${1:-check}

# ---- enumerate [features] from Cargo.toml ---------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/ {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]:-<none>})"

# ---- build the power set --------------------------------------------------
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  COMBOS=()
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "valid feature combinations to verify: ${#COMBOS[@]}"

rc=0
run() {
  echo "--- $* ---"
  if ! timeout 600 "$@"; then
    echo "FAILED: $*"
    rc=1
  fi
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    label="<no features>"
    args=(--no-default-features)
  else
    label="$combo"
    args=(--no-default-features --features "$combo")
  fi
  echo "=============================================================="
  echo "combination: $label"
  echo "=============================================================="
  run cargo check --offline "${args[@]}" --all-targets
  if [ "$MODE" = test ]; then
    run cargo build --offline "${args[@]}"
    run cargo build --offline --release "${args[@]}"
    run cargo test --offline "${args[@]}"
    run cargo test --offline --release "${args[@]}"
  fi
done

# the default and all-features configurations, for completeness
echo "=============================================================="
echo "combination: <default>"
echo "=============================================================="
run cargo check --offline --all-targets
echo "=============================================================="
echo "combination: --all-features"
echo "=============================================================="
run cargo check --offline --all-features --all-targets
if [ "$MODE" = test ]; then
  run cargo test --offline --all-features
  run cargo test --offline --release --all-features
fi

if [ "$rc" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS OK"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$rc"
