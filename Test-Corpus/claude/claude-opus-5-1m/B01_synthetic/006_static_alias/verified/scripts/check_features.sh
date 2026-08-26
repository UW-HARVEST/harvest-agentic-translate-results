#!/usr/bin/env bash
# Phase A/D: enumerate every valid build-time feature combination from
# Cargo.toml's [features] table and run `cargo check` (and optionally
# `cargo test`) for each of them, plus the empty combination.
#
# Usage: scripts/check_features.sh [check|test]   (default: check)
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
MODE="${1:-check}"

# Feature names declared in [features] (excluding "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]:-none})"

n=${#FEATURES[@]}
combos=()
if [ "$n" -eq 0 ]; then
  combos=("")
else
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (( (mask >> b) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[b]}"
      fi
    done
    combos+=("$combo")
  done
fi

fail=0
for combo in "${combos[@]}"; do
  label="${combo:-<none>}"
  printf '=== %s --no-default-features --features "%s"\n' "$MODE" "$label"
  if ! timeout 600 cargo "$MODE" --offline --no-default-features --features "$combo" \
        --all-targets 2>&1 | tail -5; then
    echo "!!! FAILED: $label"
    fail=1
  fi
done

# The default feature set is a valid configuration too.
printf '=== %s (default features)\n' "$MODE"
if ! timeout 600 cargo "$MODE" --offline --all-targets 2>&1 | tail -5; then
  echo "!!! FAILED: default"
  fail=1
fi
printf '=== %s --all-features\n' "$MODE"
if ! timeout 600 cargo "$MODE" --offline --all-features --all-targets 2>&1 | tail -5; then
  echo "!!! FAILED: --all-features"
  fail=1
fi

exit "$fail"
