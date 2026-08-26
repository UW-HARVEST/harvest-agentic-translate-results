#!/usr/bin/env bash
# Full verification driver: every feature combination x every profile,
# plus the nm -D symbol-parity diff.
#
# Usage: ./verify.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
C_SO="$ROOT/c_src/build/libtranslated_rust.so"
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf '!! FAIL: %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------- feature combos
# Enumerate every valid feature combination from Cargo.toml mechanically:
# the powerset of the [features] table (excluding "default").
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{
        sub(/[[:space:]]*=.*/,""); if ($0 != "default") print }' Cargo.toml
)
COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  COMBOS=("")            # no [features] table -> the empty set is the only combo
else
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( b=0; b<n; b++ )); do
      (( mask & (1<<b) )) && combo="${combo:+$combo,}${FEATURES[$b]}"
    done
    COMBOS+=("$combo")
  done
fi
step "feature combinations (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do echo "  --no-default-features --features '${c}'"; done

# ---------------------------------------------------------------------- C library
# (1) the default configuration, exactly as documented in the task
step "build C shared library (default configuration)"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || fail "C build (default)"
[[ -f "$C_SO" ]] || fail "missing $C_SO"

# (2) an optimized build of the SAME, unmodified C source. `*(double *)&result`
#     is a type-pun, so -O2 could in principle behave differently from -O0;
#     differential-testing against both rules that out. Built out-of-tree so
#     nothing under c_src/ is modified.
C_SO_O2="$ROOT/target/c_build_o2/libtranslated_rust.so"
step "build C shared library (-O2 / CMAKE_BUILD_TYPE=Release)"
( mkdir -p target/c_build_o2 && cd target/c_build_o2 \
  && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
       -DCMAKE_BUILD_TYPE=Release >/dev/null \
  && cmake --build . >/dev/null ) || fail "C build (-O2)"
[[ -f "$C_SO_O2" ]] || fail "missing $C_SO_O2"

# ------------------------------------------------------- check / build / test all
for combo in "${COMBOS[@]}"; do
  label="${combo:-<empty/default>}"

  step "cargo check  [$label]"
  timeout 600 cargo check --no-default-features --features "$combo" 2>&1 | tail -3 \
    || fail "cargo check [$label]"

  for profile in debug release; do
    step "build+test  [$label] [$profile]"
    if [[ $profile == release ]]; then
      RELFLAG=(--release); OUT="$ROOT/target/release/libnext_double_lib.so"
    else
      RELFLAG=();          OUT="$ROOT/target/debug/libnext_double_lib.so"
    fi

    # cargo does NOT rebuild a cdylib-only lib target for `cargo test`,
    # so build the .so explicitly first.
    timeout 600 cargo build "${RELFLAG[@]}" --no-default-features --features "$combo" \
      2>&1 | tail -2 || fail "cargo build [$label][$profile]"
    [[ -f "$OUT" ]] || { fail "missing $OUT"; continue; }

    step "symbol parity  [$label] [$profile]"
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$OUT"   | awk '{print $NF}' | sort -u))
    if [[ -n $missing ]]; then
      fail "symbols exported by C but not by Rust [$label][$profile]:"$'\n'"$missing"
    else
      echo "  OK: 0 C symbols missing from the Rust .so"
    fi
    echo "  C   : $(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u | tr '\n' ' ')"
    echo "  Rust: $(nm -D --defined-only "$OUT" | awk '{print $NF}' | sort -u | tr '\n' ' ')"

    for cvariant in default O2; do
      [[ $cvariant == default ]] && CSO="$C_SO" || CSO="$C_SO_O2"
      step "differential tests  [$label] [rust:$profile] [c:$cvariant]"
      out=$(HARVEST_RUST_SO="$OUT" HARVEST_C_SO="$CSO" timeout 600 \
              cargo test "${RELFLAG[@]}" --no-default-features \
              --features "$combo" 2>&1)
      rc=$?
      echo "$out" | grep -E 'test result:|FAILED|panicked' | sed 's/^/  /'
      (( rc == 0 )) || { echo "$out" | tail -30; \
        fail "cargo test [$label][rust:$profile][c:$cvariant]"; }
    done
  done
done

step "RESULT"
if (( FAIL )); then echo "VERIFICATION FAILED"; exit 1; fi
echo "ALL CHECKS PASSED"
