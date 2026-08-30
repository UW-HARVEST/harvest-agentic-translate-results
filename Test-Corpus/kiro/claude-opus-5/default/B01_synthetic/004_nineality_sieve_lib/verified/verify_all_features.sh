#!/usr/bin/env bash
# Enumerate every build-time configuration and check + test each one.
#
# Cargo features are read from translation/Cargo.toml. If the crate declares no
# [features] section (the current state), the only valid configuration is the
# empty feature set, and the sweep degenerates to a single pass -- which is
# still run explicitly so that adding a feature later is picked up
# automatically.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

TIMEOUT=${TIMEOUT:-600}

# --- enumerate features -----------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

# --- build the power set of features ---------------------------------------
COMBOS=("")
for f in "${FEATURES[@]}"; do
  for existing in "${COMBOS[@]}"; do
    if [ -z "$existing" ]; then COMBOS+=("$f"); else COMBOS+=("$existing,$f"); fi
  done
done
# `default` is a configuration in its own right.
COMBOS+=("__default__")

echo "Declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"
echo "Configurations to verify: ${#COMBOS[@]}"
echo

fail=0
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "__default__" ]; then
    args=()
    label="default features"
  elif [ -z "$combo" ]; then
    args=(--no-default-features)
    label="no features"
  else
    args=(--no-default-features --features "$combo")
    label="features: $combo"
  fi

  echo "=============================================================="
  echo ">>> $label"
  echo "=============================================================="

  # The harness rebuilds the cdylib under test itself; forward the same feature
  # selection so the .so being compared matches the configuration.
  export SIEVE_TEST_CARGO_ARGS="${args[*]}"

  for profile in "" "--release"; do
    pname=${profile:-dev}
    echo "--- cargo check ($pname) ---"
    if ! timeout "$TIMEOUT" cargo check --all-targets "${args[@]}" $profile 2>&1 | tail -5; then
      echo "CHECK FAILED: $label ($pname)"; fail=1
    fi
    echo "--- cargo test ($pname) ---"
    if ! timeout "$TIMEOUT" cargo test "${args[@]}" $profile 2>&1 | grep -E "^test |test result:|error"; then
      echo "TEST FAILED: $label ($pname)"; fail=1
    fi
  done
  echo
done

if [ "$fail" -ne 0 ]; then
  echo "SWEEP RESULT: FAILURES PRESENT"; exit 1
fi
echo "SWEEP RESULT: all ${#COMBOS[@]} configuration(s) x 2 profile(s) passed"
