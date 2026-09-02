#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination.
#
# Feature names are extracted from Cargo.toml rather than hardcoded, so this
# keeps working if features are added later. With no [features] section the
# powerset is just the empty set, which is covered as the default build plus an
# explicit --no-default-features run.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

TIMEOUT=${TIMEOUT:-600}
fail=0

# --- enumerate features -----------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/       { in_f=1; next }
    /^\[/                 { in_f=0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "features found: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the powerset of feature combinations ------------------------------
COMBOS=()
n=${#FEATURES[@]}
total=$((1 << n))
for ((mask = 0; mask < total; mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then
      combo="${combo:+$combo,}${FEATURES[$i]}"
    fi
  done
  COMBOS+=("$combo")
done

run() {
  local label="$1"; shift
  # The harness builds the cdylib itself (cargo test does not build a
  # crate-type=["cdylib"] artifact), so it must be told which features to use.
  local so_features="${HARVEST_SO_FEATURES:-}"
  echo
  echo "=============================================================="
  echo "== $label"
  echo "==   cargo $*"
  echo "==   HARVEST_SO_FEATURES='${so_features}'"
  echo "=============================================================="
  local log
  log=$(mktemp)
  if HARVEST_SO_FEATURES="$so_features" timeout "$TIMEOUT" cargo "$@" >"$log" 2>&1; then
    grep -E '^(     Running|test result:)' "$log" | sed 's/^/   /'
    echo "-- PASS: $label"
  else
    grep -E '^(     Running|test result:|error|thread .* panicked|assertion)' "$log" \
      | sed 's/^/   /' | head -n 40
    echo "-- FAIL: $label"
    fail=1
  fi
  rm -f "$log"
}

# Default build (whatever `default` resolves to).
HARVEST_SO_FEATURES="" run "default features" test -- --test-threads=1

# Every explicit combination, with default features disabled.
for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    HARVEST_SO_FEATURES="--no-default-features" \
      run "--no-default-features" test --no-default-features -- --test-threads=1
  else
    HARVEST_SO_FEATURES="--no-default-features --features $combo" \
      run "--no-default-features --features $combo" \
      test --no-default-features --features "$combo" -- --test-threads=1
  fi
done

# The release profile differs materially (panic = "abort", optimizations on), so
# verify it too: the exported ABI and every result must be identical. The harness
# builds the cdylib with --release automatically when debug_assertions are off.
HARVEST_SO_FEATURES="" run "release profile" test --release -- --test-threads=1

# Also run the default profile with the default (parallel) test threads, to
# confirm nothing in the harness depends on serial execution.
HARVEST_SO_FEATURES="" run "default features, parallel threads" test

echo
if ((fail)); then
  echo "RESULT: at least one configuration FAILED"
  exit 1
fi
echo "RESULT: all configurations PASSED"
