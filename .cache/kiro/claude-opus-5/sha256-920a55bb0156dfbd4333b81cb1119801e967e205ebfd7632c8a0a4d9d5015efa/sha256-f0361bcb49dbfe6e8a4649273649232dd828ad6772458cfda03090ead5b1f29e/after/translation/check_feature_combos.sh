#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination and
# EVERY build profile, plus the nm -D symbol-parity check.
#
# `cargo test` does not build a `cdylib` artifact, so each profile's `.so` is
# built explicitly and handed to the harness via RUST_DRIVER_SO.
set -uo pipefail

cd "$(dirname "$0")"
CRATE_ROOT="$PWD"
C_SO="$CRATE_ROOT/../c_src/build/libdriver.so"
FAIL=0
TIMEOUT=${TIMEOUT:-600}

# --- 0. the C library must exist ------------------------------------------
if [[ ! -f "$C_SO" ]]; then
  echo "FATAL: $C_SO missing — build it with:"
  echo "  cd ../c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi

# --- 1. enumerate feature combinations from Cargo.toml ---------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 ]]; then
  echo "== Cargo.toml declares no [features]: the feature cross-product is the single default configuration =="
  COMBOS+=("--no-default-features")
  COMBOS+=("")
  COMBOS+=("--all-features")
else
  # Full powerset of the declared features, with and without defaults.
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && sel+=("${FEATURES[$b]}")
    done
    if [[ ${#sel[@]} -eq 0 ]]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(
        IFS=,
        echo "${sel[*]}"
      )")
    fi
  done
  COMBOS+=("")
  COMBOS+=("--all-features")
fi

echo "== ${#COMBOS[@]} feature combination(s) x 2 profiles =="

# --- 2. per combination: cargo check, build the .so, symbol parity, tests --
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    label="features='${combo:-<default>}' profile=$profile"
    if [[ $profile == release ]]; then
      prof_flag="--release"
      so="$CRATE_ROOT/target/release/libdriver.so"
    else
      prof_flag=""
      so="$CRATE_ROOT/target/debug/libdriver.so"
    fi

    echo
    echo "----- $label -----"

    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo check $combo $prof_flag >/tmp/fc-check.log 2>&1; then
      echo "FAIL cargo check ($label)"
      tail -20 /tmp/fc-check.log
      FAIL=1
      continue
    fi
    echo "ok   cargo check"

    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo build $combo $prof_flag >/tmp/fc-build.log 2>&1; then
      echo "FAIL cargo build ($label)"
      tail -20 /tmp/fc-build.log
      FAIL=1
      continue
    fi
    if [[ ! -f $so ]]; then
      echo "FAIL cdylib not produced at $so ($label)"
      FAIL=1
      continue
    fi
    echo "ok   cargo build -> $(basename "$so")"

    # symbol parity for THIS .so
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$so" | awk '{print $3}' | sort -u))
    if [[ -n $missing ]]; then
      echo "FAIL symbols missing from the Rust .so ($label):"
      echo "$missing" | sed 's/^/       /'
      FAIL=1
    else
      echo "ok   nm -D parity: 0 missing symbols"
    fi

    # shellcheck disable=SC2086
    if ! RUST_DRIVER_SO="$so" timeout "$TIMEOUT" cargo test $combo $prof_flag \
      --no-fail-fast -- --test-threads=1 >/tmp/fc-test.log 2>&1; then
      echo "FAIL cargo test ($label)"
      grep -E '^(test |error|failures:|thread)' /tmp/fc-test.log | tail -40
      FAIL=1
      continue
    fi
    grep -E '^test result:' /tmp/fc-test.log | sed 's/^/ok   /'
  done
done

echo
if [[ $FAIL -eq 0 ]]; then
  echo "ALL FEATURE COMBINATIONS AND PROFILES PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $FAIL
