#!/usr/bin/env bash
# Phase D: run the whole differential suite under EVERY feature combination and
# under both cargo profiles (debug enables overflow checks and debug_assertions,
# release enables optimisation and `panic = "abort"` in the cdylib, so the two
# are genuinely different builds of the code under test).
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

# Enumerate declared features straight out of Cargo.toml (the [features] table,
# minus the implicit "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblk=1; next }
    /^\[/           { inblk=0 }
    inblk && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); if ($0 != "default") print
    }
  ' Cargo.toml
)

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of feature sets to test: the powerset of declared features,
# plus the default build.
COMBOS=("<default>")
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    set=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then set="${set:+$set,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$set")
  done
else
  # No optional features exist, so --no-default-features is the only other
  # configuration of the crate that can be selected.
  COMBOS+=("")
fi

fail=0
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    if [[ "$combo" == "<default>" ]]; then
      args=()
      label="default"
    else
      args=(--no-default-features)
      [[ -n "$combo" ]] && args+=(--features "$combo")
      label="no-default-features${combo:+ +$combo}"
    fi
    printf '=== profile=%-9s features=%-32s ' "${profile:-debug}" "$label"
    if out=$(timeout 600 cargo test ${profile:+$profile} "${args[@]}" 2>&1); then
      echo "$out" | grep -E '^test result' | tail -1
    else
      echo "FAILED"
      echo "$out" | grep -E 'test result|FAILED|DIVERGENCE|panicked|^error' | head -20
      fail=1
    fi
  done
done

if ((fail)); then
  echo "!!! at least one configuration failed"
  exit 1
fi
echo "=== all feature/profile combinations pass ==="
