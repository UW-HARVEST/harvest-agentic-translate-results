#!/usr/bin/env bash
# Phase D: enumerate every Cargo feature and run the full differential suite for
# every combination (plus the empty one). Features are extracted from
# Cargo.toml rather than hard-coded, so a newly added feature is picked up
# automatically.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"

CARGO_FLAGS="--offline"

# Parse the [features] table: names on the left of '=' inside that section.
mapfile -t FEATURES < <(
  awk '
    /^\[/            { in_f = ($0 ~ /^\[features\]/) ; next }
    !in_f            { next }
    /^[[:space:]]*#/ { next }
    /=/              { split($0, a, "="); gsub(/[[:space:]"]/, "", a[1]);
                       if (a[1] != "" && a[1] != "default") print a[1] }
  ' Cargo.toml | sort -u
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of combinations: the empty set, then the full power set.
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then
        combo="${combo:+$combo,}${FEATURES[$b]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "combinations to verify: ${#COMBOS[@]}"

fail=0
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    label="[${combo:-<no features>}] ${profile:-debug}"
    echo
    echo "=== $label ==="
    args=(--no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")

    if ! cargo check $CARGO_FLAGS $profile "${args[@]}"; then
      echo "!!! cargo check FAILED for $label"
      fail=1
      continue
    fi
    # The cdylib must be rebuilt before the tests dlopen it; the harness has a
    # staleness guard, but build explicitly so the guard never has to fire.
    cargo build $CARGO_FLAGS $profile "${args[@]}"
    if ! cargo test $CARGO_FLAGS $profile "${args[@]}" -- --test-threads=1; then
      echo "!!! cargo test FAILED for $label"
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} combo(s) x 2 profiles)"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$fail"
