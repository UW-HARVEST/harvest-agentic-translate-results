#!/usr/bin/env bash
# Enumerate every cargo feature combination declared in Cargo.toml and run the
# full differential suite (Phases B, C and D) under each one.
#
# The crate currently declares no [features] table, so the enumeration yields
# the single default configuration -- but the loop is generated from Cargo.toml,
# so adding a feature automatically widens the matrix instead of silently
# leaving new code paths untested.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CARGO_FLAGS="--offline"

# --- extract the feature names from the [features] section of Cargo.toml -----
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/        { in_f = 1; next }
    /^\[/                  { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the list of invocations ------------------------------------------
COMBOS=()
COMBOS+=("")                                  # default features
COMBOS+=("--no-default-features")             # nothing enabled
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("--all-features")
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi

echo "running ${#COMBOS[@]} configuration(s)"
echo

rc=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  printf '=== %s ===\n' "$label"

  # The Rust cdylib must exist before the tests dlopen it, and `cargo test`
  # does not build cdylib artifacts -- build explicitly first, with the same
  # feature flags so the .so under test matches the configuration.
  # shellcheck disable=SC2086
  if ! cargo build $CARGO_FLAGS $combo 2>&1 | grep -E "^(error|warning: unused)" ; then :; fi
  # shellcheck disable=SC2086
  if ! cargo build $CARGO_FLAGS $combo >/dev/null 2>&1; then
    echo "  BUILD FAILED"
    rc=1
    continue
  fi

  # shellcheck disable=SC2086
  out=$(cargo test $CARGO_FLAGS $combo -- --test-threads=1 2>&1)
  if [ $? -ne 0 ]; then
    echo "$out" | tail -40
    echo "  TESTS FAILED"
    rc=1
  else
    echo "$out" | grep -E "^test result:"
    echo "  OK"
  fi
  echo
done

# --- release profile too (different codegen, panic=abort) -------------------
echo "=== <default features> --release ==="
if cargo build $CARGO_FLAGS --release >/dev/null 2>&1 &&
  out=$(cargo test $CARGO_FLAGS --release -- --test-threads=1 2>&1); then
  echo "$out" | grep -E "^test result:"
  echo "  OK"
else
  echo "${out:-}" | tail -40
  echo "  RELEASE TESTS FAILED"
  rc=1
fi

echo
[ $rc -eq 0 ] && echo "ALL CONFIGURATIONS PASSED" || echo "SOME CONFIGURATIONS FAILED"
exit $rc
