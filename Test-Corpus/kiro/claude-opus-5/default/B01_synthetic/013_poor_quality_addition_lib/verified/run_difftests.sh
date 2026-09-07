#!/usr/bin/env bash
# Run the C-vs-Rust differential test suite across every build configuration.
#
# `cargo test` does NOT build a cdylib-only lib target, so the cdylib MUST be
# built explicitly first for each configuration; the test harness additionally
# refuses to run against a .so older than the sources.
set -uo pipefail

cd "$(dirname "$0")"

# --- 1. (Re)build the C ground-truth shared library -------------------------
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# --- 2. Enumerate feature combinations from Cargo.toml ----------------------
# Cargo.toml declares no [features] table, so the meaningful invocations are the
# default one, --no-default-features and --all-features. This is derived, not
# assumed: if a [features] table is ever added, the loop below picks up the
# powerset of the declared feature names.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{print $1}' Cargo.toml
)

declare -a COMBOS=("" "--no-default-features" "--all-features")
if (( ${#FEATURES[@]} > 0 )); then
  n=${#FEATURES[@]}
  for (( mask=0; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}"); done
    if (( ${#sel[@]} )); then
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
fi

FAIL=0
for PROFILE in "" "--release"; do
  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile='${PROFILE:-dev}' features='${COMBO:-default}'"
    echo "=============================================================="
    echo ">>> $LABEL"
    echo "=============================================================="

    # Build the cdylib under test for THIS configuration.
    if ! timeout 600 cargo build --lib $PROFILE $COMBO 2>&1 | tail -3; then
      echo "!!! BUILD FAILED: $LABEL"; FAIL=1; continue
    fi

    if ! timeout 600 cargo test $PROFILE $COMBO 2>&1 \
        | grep -E 'running |^test |test result|error|panicked|FAILED'; then
      : # grep found nothing to print; the exit code below is what matters
    fi
    if ! timeout 600 cargo test $PROFILE $COMBO >/dev/null 2>&1; then
      echo "!!! TESTS FAILED: $LABEL"; FAIL=1
    fi
  done
done

echo "=============================================================="
if (( FAIL )); then echo "RESULT: FAILURES PRESENT"; else echo "RESULT: ALL CONFIGURATIONS PASS"; fi
exit $FAIL
