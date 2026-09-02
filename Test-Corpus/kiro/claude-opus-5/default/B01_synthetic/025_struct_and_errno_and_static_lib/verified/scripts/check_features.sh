#!/usr/bin/env bash
# Phase D driver: run the FULL differential suite for EVERY cargo feature
# combination and for both the release and the debug Rust .so.
#
# The feature set is derived mechanically from Cargo.toml, not hard-coded.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(pwd)"
C_SO="$ROOT/../c_src/build/libdriver.so"

if [[ ! -f "$C_SO" ]]; then
  echo "building the C .so first"
  (cd "$ROOT/../c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || exit 1
fi

# --- enumerate features mechanically -----------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "");
      if ($0 != "default") print
    }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# Build the combination list: always the default build and the
# --no-default-features build; plus every subset of the declared features.
COMBOS=()
COMBOS+=("")                         # default features
COMBOS+=("--no-default-features")
n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask=0; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    if (( ${#sel[@]} > 0 )); then
      joined=$(IFS=,; echo "${sel[*]}")
      COMBOS+=("--no-default-features --features $joined")
      COMBOS+=("--features $joined")
    fi
  done
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  flags="$combo"
  for profile in release debug; do
    echo "=============================================================="
    echo ">>> combo: '${combo:-<default>}'   profile: ${profile}"
    echo "=============================================================="

    if [[ "$profile" == release ]]; then
      # shellcheck disable=SC2086
      timeout 600 cargo build --release $flags >/dev/null 2>&1 || { echo "BUILD FAILED"; FAIL=1; continue; }
      SO="$ROOT/target/release/libdriver.so"
    else
      # shellcheck disable=SC2086
      timeout 600 cargo build $flags >/dev/null 2>&1 || { echo "BUILD FAILED"; FAIL=1; continue; }
      SO="$ROOT/target/debug/libdriver.so"
    fi

    if [[ ! -f "$SO" ]]; then
      echo "MISSING $SO"; FAIL=1; continue
    fi

    # Symbol diff, independent of the test suite.
    diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only "$SO"   | awk '{print $NF}' | sort -u) \
      && echo "symbol diff: EMPTY (ok)" \
      || { echo "SYMBOL DIFF NON-EMPTY"; FAIL=1; }

    # shellcheck disable=SC2086
    DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$SO" \
      timeout 600 cargo test $flags -- --test-threads=1 2>&1 \
      | grep -E "^(running|test result|test .*FAILED|error)" \
      || true

    # shellcheck disable=SC2086
    DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$SO" \
      timeout 600 cargo test $flags -- --test-threads=1 >/dev/null 2>&1 \
      || { echo "TESTS FAILED for combo '${combo}' profile ${profile}"; FAIL=1; }
  done
done

echo "=============================================================="
if (( FAIL )); then
  echo "RESULT: FAILURES PRESENT"
  exit 1
fi
echo "RESULT: all feature combinations x profiles PASSED"
