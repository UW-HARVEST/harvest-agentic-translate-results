#!/usr/bin/env bash
# Enumerate every valid feature combination from Cargo.toml and run a command
# for each. Usage: ./check_all_features.sh check|test
set -uo pipefail
cd "$(dirname "$0")"

MODE="${1:-check}"

# Extract feature names from the [features] table (ignore "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "" && a[1] !~ /^#/) print a[1] }
  ' Cargo.toml
)

echo "Discovered features: ${FEATURES[*]:-<none>}"

# Build the list of combinations: always include the default build and the
# no-default-features build, plus the powerset of explicit features.
COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  COMBOS=("")
else
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then
        [ -n "$combo" ] && combo="$combo,"
        combo="$combo${FEATURES[b]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

status=0

run() {
  local desc="$1"; shift
  echo "=============================================================="
  echo ">>> $desc"
  echo "    $*"
  if ! timeout 600 "$@" > /tmp/feat.log 2>&1; then
    echo "!!! FAILED: $desc"
    tail -n 40 /tmp/feat.log
    status=1
  else
    tail -n 3 /tmp/feat.log
  fi
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    run "no-default-features (empty)" cargo "$MODE" --no-default-features
  else
    run "no-default-features + $combo" cargo "$MODE" --no-default-features --features "$combo"
  fi
done

# Also the plain default build.
run "default features" cargo "$MODE"
# And all-features, which is a valid configuration too.
run "all-features" cargo "$MODE" --all-features

exit $status
