#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and cargo-check each one.
#
# The crate declares no [features] table, so the only configuration is the
# empty (default) one -- but the enumeration is derived from Cargo.toml rather
# than hard-coded, so it stays correct if features are added later.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

# Feature names = keys of the [features] table, minus "default".
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# All subsets of FEATURES (2^n).
combos=("")
for f in "${FEATURES[@]}"; do
  new=()
  for c in "${combos[@]}"; do
    new+=("$c")
    if [[ -z "$c" ]]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  combos=("${new[@]}")
done

status=0
for c in "${combos[@]}"; do
  label="${c:-<none>}"
  printf '=== cargo check --no-default-features --features %s\n' "$label"
  if ! timeout 600 cargo check --release --no-default-features --features "$c" 2>&1 | tail -5; then
    status=1
  fi
done

# The default feature set is also a valid configuration in its own right.
echo "=== cargo check (default features)"
timeout 600 cargo check --release 2>&1 | tail -5 || status=1

exit $status
