#!/usr/bin/env bash
# Phase D driver: symbol parity + the full Phase B/C suite under EVERY feature
# combination and both profiles.
#
# Usage:  ./run_all_feature_combos.sh
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
C_SO="$CRATE_DIR/../c_src/build/libdriver.so"
fail=0
CARGO_OFFLINE=${CARGO_OFFLINE:---offline}

echo "############ 0. build the C ground-truth library ############"
if [ ! -f "$C_SO" ]; then
  ( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
echo "C .so: $C_SO"

# ---------------------------------------------------------------- feature combos
# Enumerate the powerset of the [features] declared in Cargo.toml. This crate
# declares none, so the set is just {default, no-default-features}; the loop is
# written generically so it stays correct if features are ever added.
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
echo "declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

COMBOS=()
COMBOS+=("--all-features")          # widest
COMBOS+=("")                        # default features
COMBOS+=("--no-default-features")   # narrowest
n=${#FEATURES[@]}
if [ "$n" -gt 0 ] && [ "$n" -le 12 ]; then
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--no-default-features --features $joined")
  done
fi

echo
echo "############ 1. cargo check, every combo ############"
for combo in "${COMBOS[@]}"; do
  if timeout 600 cargo check $CARGO_OFFLINE $combo >/dev/null 2>&1; then
    echo "  PASS  cargo check ${combo:-<default>}"
  else
    echo "  FAIL  cargo check ${combo:-<default>}"; fail=1
  fi
done

echo
echo "############ 2. symbol parity, every combo x profile ############"
for combo in "${COMBOS[@]}"; do
  for prof in debug release; do
    pflag=""; [ "$prof" = release ] && pflag="--release"
    timeout 600 cargo build $CARGO_OFFLINE $pflag $combo >/dev/null 2>&1 \
      || { echo "  FAIL  build $prof ${combo:-<default>}"; fail=1; continue; }
    R_SO="target/$prof/libdriver.so"
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
      <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort))
    if [ -z "$missing" ]; then
      echo "  PASS  symbols $prof ${combo:-<default>} (0 missing)"
    else
      echo "  FAIL  symbols $prof ${combo:-<default>} missing: $(echo $missing)"; fail=1
    fi
  done
done

echo
echo "############ 3. differential suite, every combo x profile ############"
for combo in "${COMBOS[@]}"; do
  for prof in debug release; do
    pflag=""; [ "$prof" = release ] && pflag="--release"
    # Tell the harness which flags to use when it rebuilds the cdylib, so the
    # .so under test matches the feature set the tests were compiled with.
    out=$(DRIVER_TEST_CARGO_FLAGS="$combo" \
          timeout 600 cargo test $CARGO_OFFLINE $pflag $combo 2>&1)
    if echo "$out" | grep -qE '^test result: FAILED|error\[|error:'; then
      echo "  FAIL  tests $prof ${combo:-<default>}"
      echo "$out" | grep -E '^test .*FAILED|^test result|DIVERGENCE|error' | head -20
      fail=1
    else
      summary=$(echo "$out" | grep -E '^test result' | awk '{s+=$4} END {print s}')
      echo "  PASS  tests $prof ${combo:-<default>} ($summary tests)"
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "########## ALL FEATURE COMBINATIONS PASSED ##########"
else
  echo "########## FAILURES PRESENT (see above) ##########"
fi
exit $fail
