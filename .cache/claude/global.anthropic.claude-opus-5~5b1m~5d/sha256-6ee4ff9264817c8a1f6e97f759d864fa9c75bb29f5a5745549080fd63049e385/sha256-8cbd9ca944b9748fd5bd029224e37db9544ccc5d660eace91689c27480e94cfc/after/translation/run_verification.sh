#!/usr/bin/env bash
# Full differential verification: builds the C .so, then runs every phase
# under every feature combination declared in Cargo.toml, in both profiles.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
OFFLINE="--offline"
FAIL=0

echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
export C_SO="$ROOT/c_src/build/libdriver.so"
echo "   $C_SO"

# --- enumerate feature combinations ------------------------------------------
# Mechanically extract feature names from the [features] table (if any).
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {gsub(/ /,"");split($0,a,"=");if(a[1]!="default")print a[1]}' \
    "$HERE/Cargo.toml"
)
echo "== declared features: ${FEATURES[*]:-<none>} =="

# Build the list of cargo feature flag sets to test: the power set.
COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("")                        # default (empty) feature set
  COMBOS+=("--no-default-features")   # explicit empty set
else
  n=${#FEATURES[@]}
  total=$((1 << n))
  COMBOS+=("")                        # default features
  for ((m = 0; m < total; m++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (( (m >> b) & 1 )) && sel+=("${FEATURES[b]}")
    done
    if [ "${#sel[@]}" -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
fi

for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="profile='${profile_flag:-debug}' features='${combo:-<default>}'"
    echo
    echo "=============================================================="
    echo "== $label"
    echo "=============================================================="
    # shellcheck disable=SC2086
    ( cd "$HERE" && cargo build $OFFLINE $profile_flag $combo ) >/dev/null 2>&1 \
      || { echo "!! cargo build failed for $label"; FAIL=1; continue; }
    if [ -n "$profile_flag" ]; then
      export RUST_SO="$HERE/target/release/libdriver.so"
    else
      export RUST_SO="$HERE/target/debug/libdriver.so"
    fi
    [ -f "$RUST_SO" ] || { echo "!! missing $RUST_SO"; FAIL=1; continue; }
    echo "   RUST_SO=$RUST_SO"
    # shellcheck disable=SC2086
    ( cd "$HERE" && cargo test $OFFLINE $profile_flag $combo -- --test-threads=4 ) 2>&1 \
      | grep -E "^(test result|error|failures:|---- |test .* FAILED)"
    # shellcheck disable=SC2086
    ( cd "$HERE" && cargo test $OFFLINE $profile_flag $combo -- --test-threads=4 ) >/dev/null 2>&1 \
      || { echo "!! TESTS FAILED for $label"; FAIL=1; }
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$FAIL"
