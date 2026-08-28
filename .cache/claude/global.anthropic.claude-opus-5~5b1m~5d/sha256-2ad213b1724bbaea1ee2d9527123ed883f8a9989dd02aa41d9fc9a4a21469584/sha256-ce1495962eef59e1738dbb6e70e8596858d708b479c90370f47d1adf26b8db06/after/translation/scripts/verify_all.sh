#!/usr/bin/env bash
# Phase D — run the whole differential suite under every configuration:
#   * every feature combination (the crate declares no [features], so the three
#     possible cargo invocations all resolve to the same empty feature set, but
#     they are exercised explicitly anyway)
#   * against the release Rust .so (the shipping artifact) and against the debug
#     Rust .so (debug_assertions + integer-overflow checks enabled)
#   * with the test harness itself built both in debug and release
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)
CSO="$ROOT/../c_src/build/libdriver.so"

fail=0
run() { # run <label> <cmd...>
  local label="$1"; shift
  echo "=================================================================="
  echo ">>> $label"
  echo "    $*"
  if timeout 600 "$@" >"$ROOT/target/last-run.log" 2>&1; then
    grep -E "^(running|test result:)" "$ROOT/target/last-run.log" | sed 's/^/    /'
    echo "    PASS: $label"
  else
    echo "    FAIL: $label"
    tail -n 40 "$ROOT/target/last-run.log" | sed 's/^/    /'
    fail=1
  fi
}

# --- prerequisites ---------------------------------------------------------
if [ ! -f "$CSO" ]; then
  echo "building the C shared object"
  (mkdir -p "$ROOT/../c_src/build" && cd "$ROOT/../c_src/build" &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null) ||
    { echo "C build failed"; exit 1; }
fi

FEATURE_COMBOS=("" "--no-default-features" "--all-features")

# --- symbol parity (Phase D gate) -----------------------------------------
for combo in "${FEATURE_COMBOS[@]}"; do
  cargo build --offline --release $combo >/dev/null 2>&1 || { echo "release build failed for '$combo'"; exit 1; }
  cargo build --offline $combo >/dev/null 2>&1 || { echo "debug build failed for '$combo'"; exit 1; }
  for so in "$ROOT/target/release/libdriver.so" "$ROOT/target/debug/libdriver.so"; do
    missing=$(comm -23 \
      <(nm -D --defined-only "$CSO" | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$so" | awk '{print $NF}' | sort -u))
    if [ -n "$missing" ]; then
      echo "FAIL: symbols exported by the C .so but missing from $so:"
      echo "$missing"
      fail=1
    else
      echo "PASS: symbol parity ($(basename "$(dirname "$so")") build, features '${combo:-default}')"
    fi
  done
done

# --- the differential suite ------------------------------------------------
for combo in "${FEATURE_COMBOS[@]}"; do
  label="features '${combo:-default}'"
  RUST_DRIVER_SO="$ROOT/target/release/libdriver.so" \
    run "release tests, release .so, $label" \
    cargo test --offline --release $combo -- --test-threads=2
  RUST_DRIVER_SO="$ROOT/target/debug/libdriver.so" \
    run "release tests, DEBUG .so, $label" \
    cargo test --offline --release $combo -- --test-threads=2
  RUST_DRIVER_SO="$ROOT/target/release/libdriver.so" \
    run "debug tests, release .so, $label" \
    cargo test --offline $combo -- --test-threads=2
done

echo "=================================================================="
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
