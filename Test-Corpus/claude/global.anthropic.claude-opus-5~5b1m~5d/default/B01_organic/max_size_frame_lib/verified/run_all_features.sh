#!/usr/bin/env bash
# Phase D: run the full differential suite under EVERY feature combination and
# every profile. `cargo metadata` reports `features: {}` for this crate, so the
# combinations reduce to the three flag spellings below -- they are still run
# explicitly so that adding a feature later is caught automatically.
set -uo pipefail
cd "$(dirname "$0")"

C_BUILD=../c_src/build
if [ ! -d "$C_BUILD" ]; then
  echo "Building the C library first..."
  (mkdir -p "$C_BUILD" && cd "$C_BUILD" \
     && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
     && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi

# Enumerate declared features; fail loudly if any appear so this script is updated.
FEATS=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
        | python3 -c 'import json,sys; print(" ".join(json.load(sys.stdin)["packages"][0]["features"]))')
if [ -n "$FEATS" ]; then
  echo "NOTE: crate declares features: $FEATS -- extend the COMBOS list below."
fi

COMBOS=("" "--no-default-features" "--all-features")
PROFILES=("" "--release")

rc_total=0
for combo in "${COMBOS[@]}"; do
  for prof in "${PROFILES[@]}"; do
    label="cargo test ${prof:-<debug>} ${combo:-<default features>}"
    # Propagate the feature selection to the nested cdylib build in the harness.
    export DIFFTEST_CARGO_FEATURE_ARGS="$combo"
    out=$(timeout 600 cargo test $prof $combo 2>&1)
    rc=$?
    passed=$(printf '%s\n' "$out" | grep -E '^test result:' \
             | python3 -c 'import sys; print(sum(int(l.replace(";","").split()[3]) for l in sys.stdin))' 2>/dev/null || echo 0)
    failed=$(printf '%s\n' "$out" | grep -E '^test result:' \
             | python3 -c 'import sys; print(sum(int(l.replace(";","").split()[5]) for l in sys.stdin))' 2>/dev/null || echo 0)
    if [ "$rc" -eq 0 ]; then
      echo "PASS | $label | $passed tests"
    else
      echo "FAIL | $label | exit=$rc passed=$passed failed=$failed"
      printf '%s\n' "$out" | grep -E 'DIVERGENCE|panicked|SIGABRT|^error' | head -10
      rc_total=1
    fi
    unset DIFFTEST_CARGO_FEATURE_ARGS
  done
done

echo
if [ "$rc_total" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS x PROFILES PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit $rc_total
