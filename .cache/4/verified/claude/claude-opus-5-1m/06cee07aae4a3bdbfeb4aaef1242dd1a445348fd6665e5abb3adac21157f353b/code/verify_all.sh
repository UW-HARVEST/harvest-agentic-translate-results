#!/usr/bin/env bash
# Phase D driver: enumerate every build-time configuration and run the full
# differential suite (Phases B, C and D) in each one.
#
#   ./verify_all.sh
#
# Configurations = (powerset of Cargo.toml [features]) x (Rust cdylib profile).
# The cdylib profile matters because the tests dlopen the *built* library, so
# both the optimized and the unoptimized lowering of the float code get checked.
set -uo pipefail
cd "$(dirname "$0")"

# --- enumerate features ----------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "features declared in Cargo.toml: $n ${FEATURES[*]:-(none)}"

COMBOS=()
if [ "$n" -eq 0 ]; then
  COMBOS=("")                       # the single, featureless configuration
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "feature combinations to verify: ${#COMBOS[@]}"

# --- build the C reference once -------------------------------------------
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "FATAL: C build failed"; exit 1; }
echo "C reference library built"

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"

  # Phase A step 2: the crate must compile in this configuration.
  echo "=== cargo check --no-default-features --features '$combo' ==="
  if [ -z "$combo" ]; then
    timeout 600 cargo check --offline --no-default-features --all-targets
  else
    timeout 600 cargo check --offline --no-default-features --features "$combo" --all-targets
  fi
  if [ $? -ne 0 ]; then
    echo "FAIL: cargo check for [$label]"
    fail=1
    continue
  fi

  # Phases B + C + D in this configuration, against both cdylib profiles.
  for profile in release debug; do
    echo "=== cargo test  features=[$label]  cdylib profile=$profile ==="
    if [ -z "$combo" ]; then
      SP_FEATURES="" SP_RUST_PROFILE="$profile" \
        timeout 600 cargo test --offline --no-default-features -- --test-threads=4
    else
      SP_FEATURES="$combo" SP_RUST_PROFILE="$profile" \
        timeout 600 cargo test --offline --no-default-features --features "$combo" -- --test-threads=4
    fi
    if [ $? -ne 0 ]; then
      echo "FAIL: cargo test for [$label] profile=$profile"
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED (${#COMBOS[@]} feature combination(s) x 2 cdylib profiles)"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
