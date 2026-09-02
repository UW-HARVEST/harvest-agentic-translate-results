#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination
# and both profiles.
#
# Feature combinations are extracted from Cargo.toml rather than hardcoded, so
# this script stays correct if features are ever added.
set -uo pipefail
cd "$(dirname "$0")/.."

C_SO=../c_src/build/libdriver.so
if [[ ! -f $C_SO ]]; then
  echo "building the C reference library..."
  (cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null)
fi

# --- enumerate declared features -------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblock = 1; next }
    /^\[/           { inblock = 0 }
    inblock && /=/  { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)
n=${#FEATURES[@]}
echo "declared features: ${n} (${FEATURES[*]:-none})"

# Combination list, one entry per line, each holding the literal cargo flags.
COMBOS=("" "--no-default-features")
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    sel=()
    for (( i = 0; i < n; i++ )); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

FAILED=0
for profile in "" "--release"; do
  for flags in "${COMBOS[@]}"; do
    label="profile=${profile:-dev} features=${flags:-default}"
    echo
    echo "================================================================"
    echo ">>> cargo test ${profile} ${flags}"
    echo "================================================================"
    # DRIVER_TEST_CARGO_FLAGS makes the test build the cdylib with the same
    # feature selection it was itself compiled with.
    # shellcheck disable=SC2086
    DRIVER_TEST_CARGO_FLAGS="${flags}" timeout 600 \
      cargo test ${profile} ${flags} 2>&1 | tee /tmp/driver-difftest-run.log \
      | grep -E 'PHASE|checks (passed|failed)|^FAIL|ALL PHASES|FAILURES PRESENT|^error'
    if grep -q 'ALL PHASES PASSED' /tmp/driver-difftest-run.log; then
      echo "OK: ${label}"
    else
      echo "!!! FAILED: ${label}"
      FAILED=1
    fi
  done
done

echo
if (( FAILED )); then
  echo "RESULT: at least one configuration FAILED"
  exit 1
fi
echo "RESULT: all configurations passed"
