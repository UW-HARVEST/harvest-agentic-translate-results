#!/usr/bin/env bash
# Enumerate every valid feature combination from Cargo.toml and run the given
# cargo subcommand (default: check) for each one.
#
# The crate declares an empty `default` feature and no others (mirroring the C
# library, which has no conditional compilation), so the enumerated power set is
# {} -- exercised as --no-default-features, default, and --all-features.
set -u
cd "$(dirname "$0")" || exit 1

CMD=${1:-check}
shift || true

# Extract feature names from the [features] section, ignoring `default`.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[a-zA-Z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

# Build the power set of non-default features.
COMBOS=("")
for f in $FEATURES; do
  new=()
  for c in "${COMBOS[@]}"; do
    new+=("$c")
    if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS=("${new[@]}")
done

fail=0
run() {
  local label="$1"; shift
  echo "=============================================================="
  echo ">>> cargo $CMD $* ($label)"
  echo "=============================================================="
  if timeout 600 cargo "$CMD" "$@" "${EXTRA[@]}"; then
    echo "--- PASS: $label"
  else
    echo "--- FAIL: $label"
    fail=1
  fi
}

EXTRA=("$@")

for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then
    run "no-default-features (empty feature set)" --no-default-features
  else
    run "no-default-features + $c" --no-default-features --features "$c"
  fi
done
run "default features"
run "all features" --all-features

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (cargo $CMD)"
else
  echo "SOME FEATURE COMBINATIONS FAILED (cargo $CMD)"
fi
exit "$fail"
