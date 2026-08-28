#!/usr/bin/env bash
# Phase D driver: runs the whole differential suite across every feature
# combination and both cargo profiles, then re-checks `nm -D` symbol parity.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=$(ls ../c_src/build/lib*.so 2>/dev/null | head -1)
if [ -z "$C_SO" ]; then
  echo "building C .so"
  ( cd ../c_src && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null )
  C_SO=$(ls ../c_src/build/lib*.so | head -1)
fi
echo "C .so: $C_SO"

# ---- enumerate feature combinations declared in Cargo.toml -----------------
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features] -> the only configuration is the default"
  COMBOS=("--no-default-features" "")
else
  COMBOS=("--no-default-features" "" "--all-features")
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
fi

FAIL=0
for PROFILE in dev release; do
  PROF_FLAG=""
  PROF_DIR="debug"
  if [ "$PROFILE" = "release" ]; then PROF_FLAG="--release"; PROF_DIR="release"; fi
  for COMBO in "${COMBOS[@]}"; do
    echo
    echo "==================================================================="
    echo "profile=$PROFILE  features='${COMBO:-<default>}'"
    echo "==================================================================="
    # shellcheck disable=SC2086
    cargo build --offline $PROF_FLAG $COMBO >/dev/null 2>&1 || { echo "BUILD FAILED"; FAIL=1; continue; }
    RS_SO="target/$PROF_DIR/libstr_put_lib.so"
    MISSING=$(comm -23 \
        <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
        <(nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort))
    if [ -n "$MISSING" ]; then
      echo "SYMBOL PARITY FAILED, missing from Rust .so:"; echo "$MISSING"; FAIL=1
    else
      echo "symbol parity: OK (0 missing)"
    fi
    # shellcheck disable=SC2086
    timeout 900 cargo test --offline $PROF_FLAG $COMBO -- --test-threads=1 2>&1 \
      | grep -E '^(test result|running|error|warning: unused)' 
    # shellcheck disable=SC2086
    timeout 900 cargo test --offline $PROF_FLAG $COMBO -- --test-threads=1 >/dev/null 2>&1 \
      || { echo "TESTS FAILED for profile=$PROFILE features='${COMBO:-<default>}'"; FAIL=1; }
  done
done

echo
if [ "$FAIL" = 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit $FAIL
