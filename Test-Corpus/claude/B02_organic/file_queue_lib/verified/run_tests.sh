#!/usr/bin/env bash
# Differential test driver.
#
#   ./run_tests.sh                     # every feature combination, all tests
#   ./run_tests.sh --test phase_b_gad  # forward extra args to `cargo test`
#
# 1. builds the C shared library (once)
# 2. enumerates every feature combination declared in Cargo.toml
# 3. for each combination: `cargo build` (so the cdylib exists for libloading)
#    then `cargo test`
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$PWD
EXTRA=("$@")
FAIL=0

# --- 1. C shared library ----------------------------------------------------
if [ ! -f c_src/build/libdriver.so ]; then
  echo "== building C shared library =="
  ( mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
echo "C  .so: $(ls -l c_src/build/libdriver.so | awk '{print $5" bytes"}')"

# --- 2. feature combinations ------------------------------------------------
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if(a[1]!="" && a[1]!="default") print a[1]}' Cargo.toml)
COMBOS=()
if [ -z "$FEATS" ]; then
  # No [features] table at all -> exactly one configuration. Run it under both
  # spellings so the --no-default-features path is exercised too.
  COMBOS=("__none__" "__default__")
else
  ARR=($FEATS)
  N=${#ARR[@]}
  for ((mask=0; mask<(1<<N); mask++)); do
    c=""
    for ((i=0;i<N;i++)); do (( (mask>>i)&1 )) && c="${c:+$c,}${ARR[i]}"; done
    COMBOS+=("${c:-__none__}")
  done
fi

echo "feature combinations: ${COMBOS[*]}"

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __none__)    FLAGS=(--no-default-features); LABEL="<none>";;
    __default__) FLAGS=();                      LABEL="<default>";;
    *)           FLAGS=(--no-default-features --features "$combo"); LABEL="$combo";;
  esac
  echo
  echo "############################################################"
  echo "## feature combination: $LABEL"
  echo "############################################################"

  echo "-- cargo check --"
  cargo check "${FLAGS[@]}" --all-targets 2>&1 | tail -5 || FAIL=1

  echo "-- cargo build (cdylib for libloading) --"
  cargo build "${FLAGS[@]}" 2>&1 | tail -3 || { echo "BUILD FAILED"; FAIL=1; continue; }
  ls -l target/debug/libdriver.so | awk '{print "RUST .so: "$5" bytes"}'

  echo "-- cargo test --"
  DRIVER_RUST_SO="$ROOT/target/debug/libdriver.so" \
    cargo test "${FLAGS[@]}" "${EXTRA[@]}" -- --test-threads=1 2>&1 \
    | tee "$ROOT/target/test-$(echo "$LABEL" | tr -d '<>,')".log \
    | grep -E "^(     Running|test result|error|failures:|---- )"
  st=${PIPESTATUS[0]}
  [ "$st" -ne 0 ] && FAIL=1
done

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit $FAIL
