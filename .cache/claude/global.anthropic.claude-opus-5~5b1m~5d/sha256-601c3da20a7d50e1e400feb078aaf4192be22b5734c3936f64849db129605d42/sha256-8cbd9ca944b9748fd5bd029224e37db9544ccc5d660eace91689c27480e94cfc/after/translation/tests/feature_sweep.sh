#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# Feature names are extracted mechanically from Cargo.toml rather than assumed.
# Both the C .so and the Rust .so are (re)built first, and the suite is run
# against the release cdylib AND the debug cdylib, since they are separate
# codegen configurations of the same source.
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
cd "$CRATE_DIR" || exit 1

CARGO_FLAGS="--offline"
FAILED=0

# ---------------------------------------------------------------- build C .so
echo "=== building C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -n1)"
echo "C .so: $C_SO"

# ------------------------------------------------- enumerate feature combos
# Lines inside the [features] table of Cargo.toml, of the form `name = [...]`.
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "=== Cargo.toml declares no [features] -> the only configurations are"
  echo "    the default build and --no-default-features (identical) ==="
  COMBOS+=("DEFAULT")
  COMBOS+=("NONE")
else
  FEAT_ARR=($FEATURES)
  N=${#FEAT_ARR[@]}
  echo "=== features found: ${FEAT_ARR[*]} -> $((1 << N)) combinations ==="
  COMBOS+=("DEFAULT")
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (( (mask >> i) & 1 )); then combo="$combo,${FEAT_ARR[$i]}"; fi
    done
    COMBOS+=("${combo#,}")
  done
fi

run_suite() {  # $1 = human label, $2.. = extra cargo flags
  local label="$1"; shift
  echo
  echo "############ $label ############"

  for profile in release debug; do
    local pflag=""
    [ "$profile" = release ] && pflag="--release"

    echo "---- building cdylib ($profile) ----"
    # shellcheck disable=SC2086
    cargo build $CARGO_FLAGS $pflag "$@" 2>&1 | tail -n 2
    local rust_so="$CRATE_DIR/target/$profile/libdiv_euclid_lib.so"
    if [ ! -f "$rust_so" ]; then
      echo "!! missing $rust_so"; FAILED=1; continue
    fi

    echo "---- nm -D symbol parity ($profile) ----"
    diff <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u) \
      && echo "symbol diff EMPTY (ok)" \
      || { echo "!! SYMBOL DIFF NON-EMPTY"; FAILED=1; }

    echo "---- differential tests ($profile .so) ----"
    # Point the harness at this specific .so; tests themselves always build in
    # the `test` profile, but they dlopen the artifact under test.
    # shellcheck disable=SC2086
    C_SO="$C_SO" RUST_SO="$rust_so" \
      EXHAUSTIVE_STRIDE="${EXHAUSTIVE_STRIDE:-65521}" \
      SWEEP_POINTS="${SWEEP_POINTS:-2000000}" \
      SOAK_SAMPLES="${SOAK_SAMPLES:-1000000}" \
      timeout 600 cargo test $CARGO_FLAGS --release "$@" 2>&1 | tail -n 40
    if [ "${PIPESTATUS[0]}" != 0 ]; then
      echo "!! TESTS FAILED for $label ($profile)"; FAILED=1
    fi
  done
}

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT) run_suite "features: <default>" ;;
    NONE)    run_suite "features: --no-default-features" --no-default-features ;;
    "")      run_suite "features: --no-default-features (empty set)" --no-default-features ;;
    *)       run_suite "features: --no-default-features --features $combo" \
                 --no-default-features --features "$combo" ;;
  esac
done

echo
if [ "$FAILED" = 0 ]; then
  echo "===== ALL FEATURE COMBINATIONS PASSED ====="
else
  echo "===== FAILURES DETECTED ====="
fi
exit "$FAILED"
