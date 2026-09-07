#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination and
# both profiles, re-checking symbol parity for each resulting cdylib.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded.
set -uo pipefail
cd "$(dirname "$0")"

fail=0
step() { echo; echo "===== $* ====="; }

# ---------------------------------------------------------------- C library
step "building the C shared library"
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
C_SO="$(ls ../c_src/build/lib*.so)"
echo "C .so: $C_SO"

# ------------------------------------------------- enumerate feature combos
# Every feature name declared under [features] (excluding "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/ {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=("--no-default-features" "")   # always test both ends
if [ "${#FEATURES[@]}" -gt 0 ]; then
  echo "declared features: ${FEATURES[*]}"
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    COMBOS+=("--features $(IFS=,; echo "${sel[*]}")")
  done
else
  echo "no [features] table in Cargo.toml -> only the default configuration exists"
fi

# ---------------------------------------------------------------- the sweep
for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="profile='${profile_flag:-dev}' features='${combo:-<default>}'"
    step "$label : cargo check"
    # shellcheck disable=SC2086
    timeout 600 cargo check $profile_flag $combo >/dev/null 2>&1 \
      || { echo "CHECK FAILED: $label"; fail=1; continue; }

    step "$label : cargo build (cdylib)"
    # shellcheck disable=SC2086
    timeout 600 cargo build $profile_flag $combo >/dev/null 2>&1 \
      || { echo "BUILD FAILED: $label"; fail=1; continue; }

    if [ -n "$profile_flag" ]; then prof=release; else prof=debug; fi
    R_SO="target/$prof/libjumpnode_lib.so"

    step "$label : symbol parity"
    diff <(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort) \
         <(nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TDBR]$/ {print $3}' | grep -v '^__' | sort) \
      && echo "symbol diff EMPTY for $label" \
      || { echo "SYMBOL PARITY FAILED: $label"; fail=1; }

    step "$label : differential test suite (phases B, C, D)"
    log="target/testlog-${prof}-$(echo "${combo:-default}" | tr -c 'A-Za-z0-9' '_').txt"
    # shellcheck disable=SC2086
    if timeout 600 cargo test $profile_flag $combo > "$log" 2>&1; then
      grep -E '^(running|test result)' "$log"
    else
      echo "TESTS FAILED: $label (see $log)"
      grep -E '^(test result|test .* FAILED|thread .* panicked|DIVERGENCE)' "$log" | head -n 30
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} combos x 2 profiles)"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
