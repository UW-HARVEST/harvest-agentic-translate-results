#!/usr/bin/env bash
# Enumerate every valid build-time configuration and check + test each one.
#
# Feature names are extracted from the [features] table in Cargo.toml, so this
# stays correct if features are added later. With N features the script visits
# all 2^N subsets (plus the default feature set).
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

TIMEOUT=${TIMEOUT:-600}
fail=0

mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /=/   { split($0, a, "="); gsub(/[ \t"]/, "", a[1]); if (a[1] != "" && a[1] !~ /^#/) print a[1] }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "== features declared in Cargo.toml: $n ${FEATURES[*]:-(none)}"

combos=()
if (( n == 0 )); then
  combos+=("")            # the only configuration there is
else
  for (( mask = 0; mask < (1 << n); mask++ )); do
    sel=()
    for (( i = 0; i < n; i++ )); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[i]}")
    done
    combos+=("$(IFS=,; echo "${sel[*]}")")
  done
fi

run() {
  local label="$1"; shift
  echo "-- $label"
  if ! timeout "$TIMEOUT" "$@" > /tmp/pow_ver.log 2>&1; then
    echo "   FAILED: $*"
    tail -n 30 /tmp/pow_ver.log
    fail=1
  else
    echo "   ok"
  fi
}

for combo in "${combos[@]}"; do
  label="${combo:-<no features>}"
  echo "=== configuration: $label"
  run "cargo check          [$label]" cargo check   --no-default-features --features "$combo"
  run "cargo check --release[$label]" cargo build --release --no-default-features --features "$combo"
  run "cargo test           [$label]" cargo test    --no-default-features --features "$combo"
done

# Also exercise the default feature set explicitly.
echo "=== configuration: <default features>"
run "cargo check  [default]" cargo check
run "cargo test   [default]" cargo test

if (( fail )); then
  echo "RESULT: FAILURES"
  exit 1
fi
echo "RESULT: all configurations check and test clean"
