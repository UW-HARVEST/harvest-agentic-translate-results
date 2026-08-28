#!/usr/bin/env bash
#
# Full verification run.
#
#   1. builds the C reference library with exactly the command from the task
#      description (no CMAKE_BUILD_TYPE => assert() is live);
#   2. enumerates every cargo feature combination declared in Cargo.toml;
#   3. for each combination, builds the cdylib in *both* cargo profiles, diffs
#      `nm -D` against the C library, and runs the differential suite:
#        - the full suite against the release cdylib (the shipped one:
#          panic=abort, overflow-checks=off), in the dev and release test
#          profiles;
#        - the non-fuzz suite against the dev cdylib as well, because that one
#          has `overflow-checks`/`debug-assertions` on and would panic on any
#          arithmetic the translation failed to make explicitly wrapping.
#
#   ./run_all.sh            # everything
#   ./run_all.sh --quick    # skip the (slow) fuzz targets
#
set -uo pipefail
cd "$(dirname "$0")"

QUICK=0
[ "${1:-}" = "--quick" ] && QUICK=1

CARGO_FLAGS="--offline"
FAST_TESTS="--test symbols_diff --test unfilter_diff --test unfilter_errors \
            --test inflate_valid --test inflate_errors"
FAIL=0

hr() { printf '=%.0s' {1..78}; echo; }
run_step() {
  local label="$1"; shift
  echo "-- $label"
  "$@" 2>&1 | grep -E "^test result|^error|FAILED|HARNESS ERROR|layout-dependent" | sed 's/^/     /'
  if [ "${PIPESTATUS[0]}" != 0 ]; then FAIL=1; echo "     ^^ STEP FAILED"; fi
}

# ---------------------------------------------------------------------------
# 1. the C reference library, built with the command from the task description
# ---------------------------------------------------------------------------
hr
echo "== building the C reference library (no CMAKE_BUILD_TYPE => assert() live)"
(
  cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "FAILED to build the C library"; exit 1; }
C_SO=$(ls ../c_src/build/lib*.so)
echo "   $C_SO"

# ---------------------------------------------------------------------------
# 2. enumerate the feature combinations mechanically from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t OPT_FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[ ]*=/{print $1}' Cargo.toml \
    | grep -v '^default$'
)
echo "== optional features: ${OPT_FEATURES[*]:-<none>}"

COMBOS=("")                                   # the default feature set
n=${#OPT_FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  sel=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then sel="$sel,${OPT_FEATURES[$i]}"; fi
  done
  if [ -z "$sel" ]; then
    COMBOS+=("--no-default-features")
  else
    COMBOS+=("--no-default-features --features ${sel#,}")
  fi
done

echo "== feature combinations to verify:"
for c in "${COMBOS[@]}"; do echo "     ${c:-<default>}"; done

# ---------------------------------------------------------------------------
# 3. per combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  hr
  echo "== FEATURES: ${combo:-<default>}"

  for prof_dir in release debug; do
    if [ "$prof_dir" = release ]; then bflag="--release"; else bflag=""; fi
    cargo build $CARGO_FLAGS $bflag $combo >/dev/null 2>&1 \
      || { echo "   cdylib BUILD FAILED ($prof_dir)"; FAIL=1; continue; }
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
      <(nm -D --defined-only "target/$prof_dir/libunfilter_lib.so" | awk '{print $3}' | sort))
    if [ -n "$missing" ]; then
      echo "   SYMBOLS MISSING FROM target/$prof_dir/libunfilter_lib.so:"
      echo "$missing" | sed 's/^/     /'
      FAIL=1
    else
      echo "   symbol diff (target/$prof_dir): empty"
    fi
  done

  # the shipped (release) cdylib, both test profiles
  if [ "$QUICK" = 1 ]; then
    run_step "release cdylib, dev tests (quick)" \
      cargo test $CARGO_FLAGS $combo $FAST_TESTS
    run_step "release cdylib, release tests (quick)" \
      cargo test $CARGO_FLAGS --release $combo $FAST_TESTS
  else
    run_step "release cdylib, dev tests" cargo test $CARGO_FLAGS $combo
    run_step "release cdylib, release tests" cargo test $CARGO_FLAGS --release $combo
  fi

  # the dev-profile cdylib: overflow-checks / debug-assertions are ON there
  TRANSLATION_SO="$PWD/target/debug/libunfilter_lib.so" \
    run_step "dev cdylib (overflow-checks on)" \
    cargo test $CARGO_FLAGS $combo $FAST_TESTS
done

hr
if [ "$FAIL" = 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES - see above"
fi
exit $FAIL
