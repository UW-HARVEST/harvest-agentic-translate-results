#!/usr/bin/env bash
# Phase D automation: run the full differential suite under EVERY feature
# combination and under both cdylib build profiles.
#
# Feature combos are extracted from Cargo.toml rather than hard-coded, so this
# keeps working if features are ever added.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
C_SO="$ROOT/../c_src/build/libdriver.so"

if [[ ! -f "$C_SO" ]]; then
  echo "FATAL: C .so missing at $C_SO"
  echo "Build it: cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi
export C_DRIVER_SO="$C_SO"

# ---- enumerate the feature powerset from Cargo.toml ------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "== declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=()
COMBOS+=("--all-features")                # superset (== default when none exist)
COMBOS+=("")                              # plain default
COMBOS+=("--no-default-features")         # minimal
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=0; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    joined=$(IFS=,; echo "${sel[*]:-}")
    COMBOS+=("--no-default-features --features $joined")
  done
fi

FAIL=0
PASSED=0

for PROFILE in debug release; do
  if [[ $PROFILE == release ]]; then BUILD_FLAG="--release"; else BUILD_FLAG=""; fi
  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features='${COMBO:-<default>}'"
    echo
    echo "=================================================================="
    echo "== $LABEL"
    echo "=================================================================="

    # Build the cdylib for this combination, then point the tests at it.
    if ! cargo build --offline $BUILD_FLAG $COMBO >/dev/null 2>&1; then
      echo "BUILD FAILED: $LABEL"; FAIL=$((FAIL+1)); continue
    fi
    SO="$ROOT/target/$PROFILE/libdriver.so"
    if [[ ! -f "$SO" ]]; then
      echo "MISSING CDYLIB for $LABEL at $SO"; FAIL=$((FAIL+1)); continue
    fi
    export RUST_DRIVER_SO="$SO"

    # Symbol parity for this exact artifact.
    if ! diff <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort) \
              <(nm -D --defined-only "$SO"    | awk '{print $NF}' | sort) \
         | grep -q '^<'; then
      echo "  symbol parity: OK (no C symbol missing from Rust)"
    else
      echo "  symbol parity: FAILED — C symbols missing from Rust:"
      comm -23 <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
               <(nm -D --defined-only "$SO"   | awk '{print $NF}' | sort) | sed 's/^/    /'
      FAIL=$((FAIL+1))
    fi

    if timeout 600 cargo test --offline $COMBO 2>&1 | tail -40; then
      echo "  TESTS: PASS ($LABEL)"; PASSED=$((PASSED+1))
    else
      echo "  TESTS: FAIL ($LABEL)"; FAIL=$((FAIL+1))
    fi
  done
done

echo
echo "=================================================================="
echo "configurations passed: $PASSED   failed: $FAIL"
echo "=================================================================="
exit $(( FAIL > 0 ? 1 : 0 ))
