#!/usr/bin/env bash
# Phase D completion gate: run the full differential suite under every feature
# combination AND both build profiles, plus the symbol-parity diff for each.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/translation" || exit 1

FAIL=0
note() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }

# ---------------------------------------------------------------------------
# 0. Build the C shared library (ground truth).
# ---------------------------------------------------------------------------
note "Building C ground-truth shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
CSO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
echo "C .so: $CSO"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
#    This crate declares no [features], so the meaningful set is the three
#    flag spellings below; the loop is written generically so it still covers
#    the full power set if features are ever added.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {gsub(/ /,""); split($0,a,"="); if (a[1] != "default") print a[1]}' Cargo.toml
)
echo "declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

COMBOS=("" "--no-default-features" "--all-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    if [ "${#sel[@]}" -gt 0 ]; then
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
fi

# ---------------------------------------------------------------------------
# 2. cargo check every combination first (fast failure on compile errors).
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  note "cargo check --tests ${combo:-(default)}"
  # shellcheck disable=SC2086
  if ! cargo check --tests $combo 2>&1 | tail -5; then FAIL=1; fi
done

# ---------------------------------------------------------------------------
# 3. Build + test every (profile x feature-combo) pair, and diff symbols for
#    the cdylib each pair produces.
# ---------------------------------------------------------------------------
for profile in debug release; do
  relflag=""; [ "$profile" = release ] && relflag="--release"
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=${combo:-(default)}"

    note "BUILD $label"
    # shellcheck disable=SC2086
    if ! cargo build $relflag $combo 2>&1 | tail -3; then
      echo "BUILD FAILED: $label"; FAIL=1; continue
    fi

    note "SYMBOL PARITY $label"
    if ! "$ROOT/scripts/symdiff.sh" "$ROOT/translation/target/$profile/libldexp_q2_lib.so"; then
      echo "SYMBOL PARITY FAILED: $label"; FAIL=1
    fi

    note "TEST $label"
    # shellcheck disable=SC2086
    if timeout 600 cargo test $relflag $combo 2>&1 \
         | grep -E 'Running|test result|FAILED|panicked|error\['; then
      echo "TESTS PASSED: $label"
    else
      echo "TESTS FAILED: $label"; FAIL=1
    fi
  done
done

note "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAIL"
