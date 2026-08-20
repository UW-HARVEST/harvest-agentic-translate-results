#!/usr/bin/env bash
# Enumerate every valid feature combination from Cargo.toml and cargo-check each.
set -uo pipefail
cd "$(dirname "$0")"

# Extract feature names from the [features] table (excluding "default").
mapfile -t FEATS < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[ ]*=/ {
    split($0, a, "="); gsub(/[ \t]/, "", a[1]);
    if (a[1] != "default") print a[1];
  }' Cargo.toml)

n=${#FEATS[@]}
echo "Optional features found: ${n} ${FEATS[*]:-(none)}"

fail=0
total=$(( 1 << n ))
for (( mask=0; mask<total; mask++ )); do
  combo=()
  for (( b=0; b<n; b++ )); do
    if (( (mask >> b) & 1 )); then combo+=("${FEATS[$b]}"); fi
  done
  joined=$(IFS=,; echo "${combo[*]}")
  label=${joined:-"<none>"}
  printf '=== cargo check --no-default-features --features "%s" ===\n' "$label"
  if cargo check --offline --all-targets --no-default-features --features "$joined" 2>&1 | tail -5; then
    echo "OK: $label"
  else
    echo "FAIL: $label"; fail=1
  fi
done
# also check the declared default feature set
echo "=== cargo check (default features) ==="
cargo check --offline --all-targets 2>&1 | tail -5 || fail=1
exit $fail
