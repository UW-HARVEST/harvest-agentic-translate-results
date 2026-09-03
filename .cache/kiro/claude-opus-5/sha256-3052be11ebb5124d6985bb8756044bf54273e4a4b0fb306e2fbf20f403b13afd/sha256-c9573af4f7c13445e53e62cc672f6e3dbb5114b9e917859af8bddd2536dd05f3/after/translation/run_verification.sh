#!/usr/bin/env bash
# Phase D driver: build the C reference, then run the whole differential suite
# under EVERY cargo feature combination and under both profiles.
#
# `translation/Cargo.toml` declares no `[features]` table, so the feature power
# set is exactly one element: the default (empty) set. That is derived here
# mechanically rather than assumed, so the script starts failing the moment a
# feature is added.
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)

echo "=== 1. C reference library ==="
(cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . 2>&1 | tail -2)
C_SO=$(ls "$ROOT"/c_src/build/*.so)
echo "C .so: $C_SO"
if ! nm -D "$C_SO" | grep -q '__assert_fail'; then
  echo "WARNING: the C library was built WITHOUT live assert()s."
  echo "         ERRORS.md rows A1-A10 assume they are live (no CMAKE_BUILD_TYPE)."
fi

echo
echo "=== 2. feature combinations (derived from Cargo.toml) ==="
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "no [features] table -> the only combination is the default (empty) set"
  COMBOS=("")
else
  echo "features found: $FEATURES"
  COMBOS=("")
  for f in $FEATURES; do
    NEW=()
    for c in "${COMBOS[@]}"; do NEW+=("$c" "${c:+$c,}$f"); done
    COMBOS=("${NEW[@]}")
  done
fi

FAIL=0
for PROFILE in release dev; do
  PROF_FLAG=""
  [ "$PROFILE" = release ] && PROF_FLAG="--release"
  for COMBO in "${COMBOS[@]}"; do
    if [ -z "$COMBO" ]; then
      FEAT_FLAG=""
      LABEL="default features"
    else
      FEAT_FLAG="--no-default-features --features $COMBO"
      LABEL="features=$COMBO"
    fi
    echo
    echo "=== profile=$PROFILE  $LABEL ==="
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $PROF_FLAG --offline $FEAT_FLAG 2>&1 | grep -E '^error' -A5; then :; fi
    # shellcheck disable=SC2086
    timeout 600 cargo test $PROF_FLAG --offline $FEAT_FLAG 2>&1 \
      | grep -E '^test [a-z_]+ \.\.\.|^test result|^error' || true
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test $PROF_FLAG --offline $FEAT_FLAG >/dev/null 2>&1; then
      echo "!! FAILURE for profile=$PROFILE $LABEL"
      FAIL=1
    fi
  done
done

echo
echo "=== 3. symbol parity ==="
R_SO=$(ls target/release/libpinflate_lib.so)
nm -D "$C_SO"  | grep -v ' U \| w ' | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D "$R_SO"  | grep -v ' U \| w ' | awk '{print $3}' | sort > /tmp/r_syms.txt
echo "C exports:    $(wc -l < /tmp/c_syms.txt)"
echo "Rust exports: $(wc -l < /tmp/r_syms.txt)"
echo "--- symbols in C but not in Rust (MUST be empty) ---"
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt | tee /tmp/missing.txt
if [ -s /tmp/missing.txt ]; then echo "!! MISSING SYMBOLS"; FAIL=1; else echo "(none)"; fi

echo
if [ "$FAIL" = 0 ]; then echo "ALL PHASES PASSED"; else echo "FAILURES PRESENT"; exit 1; fi
