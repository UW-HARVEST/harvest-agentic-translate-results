#!/usr/bin/env bash
# Runs the full C-vs-Rust differential suite for EVERY buildable configuration.
#
#   ./run-diff-tests.sh
#
# For each configuration it (1) builds the cdylib for that profile -- `cargo
# test` alone does NOT build a cdylib -- and (2) runs the tests, which dlopen
# both that object and the cmake-built C .so.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
LOGDIR="$ROOT/target/difftest-logs"
mkdir -p "$LOGDIR"

# ---------------------------------------------------------------- C library
CBUILD="$ROOT/../c_src/build"
if [ ! -d "$CBUILD" ] || ! ls "$CBUILD"/lib*.so >/dev/null 2>&1; then
  echo "== building the C shared library =="
  mkdir -p "$CBUILD"
  ( cd "$CBUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . ) \
    || { echo "FATAL: C build failed"; exit 1; }
fi
C_SO=$(ls "$CBUILD"/lib*.so | head -1)
echo "C .so: $C_SO"

# ------------------------------------------------------- feature combinations
# Cargo.toml declares no [features] table, so the feature cross-product is the
# single empty combination. All three spellings are exercised anyway, because
# they are distinct cargo resolutions.
FEATURE_SETS=(
  ""
  "--no-default-features"
  "--all-features"
)
PROFILES=("" "--release")

FAIL=0
PASS=0
declare -a RESULTS=()

for prof in "${PROFILES[@]}"; do
  for feat in "${FEATURE_SETS[@]}"; do
    label="profile=${prof:-dev} features=${feat:-default}"
    slug=$(echo "${prof:-dev}${feat:-default}" | tr -cd 'a-zA-Z0-9')
    log="$LOGDIR/$slug.log"
    echo
    echo "=================================================================="
    echo "== $label"
    echo "=================================================================="

    # shellcheck disable=SC2086
    if ! cargo build --offline $prof $feat >"$log" 2>&1; then
      echo "  BUILD FAILED (see $log)"; tail -20 "$log"; FAIL=$((FAIL+1))
      RESULTS+=("FAIL(build)  $label"); continue
    fi
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test --offline $prof $feat >>"$log" 2>&1; then
      echo "  TESTS FAILED (see $log)"
      grep -E "^test .*FAILED|panicked at|test result" "$log" | head -40
      FAIL=$((FAIL+1)); RESULTS+=("FAIL(test)   $label"); continue
    fi
    grep -E "^test result|Running" "$log" | sed 's/^/  /'
    PASS=$((PASS+1)); RESULTS+=("PASS         $label")
  done
done

echo
echo "=================================================================="
echo "== summary"
echo "=================================================================="
for r in "${RESULTS[@]}"; do echo "  $r"; done
echo "  configurations passed: $PASS   failed: $FAIL"
[ "$FAIL" -eq 0 ] || exit 1
echo "  ALL CONFIGURATIONS PASSED"
