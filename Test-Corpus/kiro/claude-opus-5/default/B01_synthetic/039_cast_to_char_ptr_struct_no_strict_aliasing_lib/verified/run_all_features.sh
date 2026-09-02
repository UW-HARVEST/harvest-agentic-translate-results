#!/usr/bin/env bash
# Phase D: run cargo check + the differential suite under EVERY feature
# combination declared in Cargo.toml (including none / --no-default-features).
set -uo pipefail
cd "$(dirname "$0")"

# Extract feature names mechanically from the [features] table, if any.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, "", $0); print }
  ' Cargo.toml | grep -v '^default$'
)

echo "declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-（none）}"

run() {
  local label="$1"; shift
  echo "=== $label ==="
  timeout 600 cargo check "$@" >/dev/null 2>&1 \
    || { echo "CHECK FAILED: $label"; return 1; }
  timeout 600 cargo test "$@" 2>&1 | tail -n 3 \
    | sed 's/^/    /' || { echo "TEST FAILED: $label"; return 1; }
}

fail=0
run "default features" || fail=1
run "no default features" --no-default-features || fail=1

# Power set of the declared features.
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && combo+=("${FEATURES[$i]}")
    done
    csv=$(IFS=,; echo "${combo[*]}")
    run "features: $csv" --no-default-features --features "$csv" || fail=1
  done
fi

echo
if (( fail )); then echo "RESULT: FAILURES PRESENT"; exit 1; fi
echo "RESULT: all feature combinations pass"
