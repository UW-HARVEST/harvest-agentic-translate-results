#!/usr/bin/env bash
# Phase D — enumerate the feature powerset from Cargo.toml and `cargo check`
# every combination. Derived mechanically, so it stays correct if features are
# added later.
set -uo pipefail
cd "$(dirname "$0")/.."

# Extract feature names from the [features] table (ignoring "default").
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }
' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features] -> exactly one configuration (default)."
  echo "=== cargo check (default) ==="
  cargo check --all-targets || exit 1
  echo "=== cargo check --no-default-features ==="
  cargo check --all-targets --no-default-features || exit 1
  echo "OK: 1 configuration verified."
  exit 0
fi

# shellcheck disable=SC2206
FEATS=($FEATURES)
N=${#FEATS[@]}
echo "features: ${FEATS[*]}"
fail=0
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if (((mask >> i) & 1)); then combo="$combo,${FEATS[$i]}"; fi
  done
  combo="${combo#,}"
  echo "=== cargo check --no-default-features --features '$combo' ==="
  cargo check --all-targets --no-default-features --features "$combo" || fail=1
done
exit $fail
