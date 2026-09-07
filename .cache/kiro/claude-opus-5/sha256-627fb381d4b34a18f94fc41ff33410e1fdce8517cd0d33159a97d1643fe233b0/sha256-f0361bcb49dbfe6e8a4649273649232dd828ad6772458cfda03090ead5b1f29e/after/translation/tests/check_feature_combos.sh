#!/usr/bin/env bash
# Phase D — enumerate every Cargo feature combination and run the full
# differential suite under each one.
#
# Usage: tests/check_feature_combos.sh [check|test]
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

MODE="${1:-test}"

# Extract feature names from the [features] table of Cargo.toml (excluding
# "default", whose members are covered by the default build).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "features found: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the combination list: default, no-default, then the power set of the
# explicit features (with --no-default-features).
COMBOS=("" "--no-default-features")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
    COMBOS+=("--features $sel")
  done
fi

status=0
for combo in "${COMBOS[@]}"; do
  flags="$combo"
  echo "=============================================================="
  echo ">>> combination: ${flags:-<default>}"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release $flags > /tmp/fc-build.log 2>&1; then
    echo "!!! release build FAILED for '${flags:-<default>}'"; tail -30 /tmp/fc-build.log; status=1; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo $MODE $flags > /tmp/fc-run.log 2>&1; then
    echo "!!! cargo $MODE FAILED for '${flags:-<default>}'"; tail -40 /tmp/fc-run.log; status=1; continue
  fi
  grep -E '^test result|running [0-9]+ test' /tmp/fc-run.log | sed 's/^/    /'
  echo ">>> OK: ${flags:-<default>}"
done
exit $status
