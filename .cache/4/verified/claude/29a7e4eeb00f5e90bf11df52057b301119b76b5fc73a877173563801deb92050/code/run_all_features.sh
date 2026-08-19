#!/usr/bin/env bash
# Phase D — run cargo check + the full differential suite for EVERY valid
# feature combination.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# this keeps working if features are ever added.
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"

# ---- enumerate features declared in Cargo.toml -----------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblock=1; next }
    /^\[/           { inblock=0 }
    inblock && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "declared features: ${#FEATURES[@]} (${FEATURES[*]-none})"

# ---- build the powerset of feature combinations -----------------------------
COMBOS=()
n=${#FEATURES[@]}
total=$((1 << n))
for ((mask = 0; mask < total; mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then
      combo="${combo:+$combo,}${FEATURES[i]}"
    fi
  done
  COMBOS+=("$combo")
done

echo "feature combinations to verify: ${#COMBOS[@]}"

# ---- ensure the C reference library exists ---------------------------------
if [[ ! -f c_src/build/libdriver.so ]]; then
  echo "building the C reference shared library"
  (mkdir -p c_src/build && cd c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi

rc=0
# DIFFTEST_BUILD_ARGS tells the test harness which feature flags to use when it
# rebuilds the cdylib under test (cargo test does not rebuild a cdylib), so the
# .so being compared always matches the configuration being tested.
run() { # run <label> <cargo args...>
  local label="$1"; shift
  echo
  echo "=============================================================="
  echo ">>> $label"
  echo "    cargo $*"
  echo "=============================================================="
  if timeout 600 cargo "$@" 2>&1 | tail -n 45; then
    echo "--- PASS: $label"
  else
    echo "--- FAIL: $label"
    rc=1
  fi
}

for combo in "${COMBOS[@]}"; do
  label="features={${combo:-<empty>}}"
  featargs="--no-default-features"
  [[ -n "$combo" ]] && featargs="$featargs --features $combo"
  export DIFFTEST_BUILD_ARGS="$featargs"
  run "check  $label" check $CARGO_FLAGS $featargs --all-targets
  run "build  $label" build $CARGO_FLAGS $featargs
  run "test   $label" test  $CARGO_FLAGS $featargs
  unset DIFFTEST_BUILD_ARGS
done

# Default feature set and --all-features (identical to {} here, but verified
# explicitly so a future `default = [...]` cannot slip through untested).
export DIFFTEST_BUILD_ARGS=""
run "check  default features"  check $CARGO_FLAGS --all-targets
run "build  default features"  build $CARGO_FLAGS
run "test   default features"  test  $CARGO_FLAGS
export DIFFTEST_BUILD_ARGS="--all-features"
run "check  --all-features"    check $CARGO_FLAGS --all-features --all-targets
run "build  --all-features"    build $CARGO_FLAGS --all-features
run "test   --all-features"    test  $CARGO_FLAGS --all-features
unset DIFFTEST_BUILD_ARGS

echo
if [[ $rc -eq 0 ]]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $rc
