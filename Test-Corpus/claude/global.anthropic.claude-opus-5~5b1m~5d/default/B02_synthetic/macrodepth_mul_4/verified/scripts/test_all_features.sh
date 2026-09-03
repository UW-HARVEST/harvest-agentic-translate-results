#!/usr/bin/env bash
# Runs Phases B + C + D for every feature combination.
#
#   bash scripts/test_all_features.sh            # 24 canonical (OP, REPEAT) builds
#   bash scripts/test_all_features.sh degenerate # + empty / conflicting sets
#
# `--test-threads=1` is required: the stdout-capture harness redirects fd 1,
# which is process-global.
set -u
cd "$(dirname "$0")/.."
export CARGO_NET_OFFLINE=true

combos=()
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    combos+=("$op,$rep")
  done
done

if [ "${1:-}" = "degenerate" ]; then
  combos+=(
    ""                       # no OP, no REPEAT -> add / 5
    "add" "sub" "mul"        # no REPEAT        -> 5
    "0" "5" "7"              # no OP            -> add
    "add,sub,5" "sub,mul,5" "add,mul,5" "add,sub,mul,5"
    "add,0,5" "add,3,5,7" "mul,0,1,2,3,4,5,6,7"
  )
fi

fail=0
for combo in "${combos[@]}"; do
  printf '=== features=[%s] ' "$combo"
  # The cdylib must exist before the tests dlopen it.
  if ! cargo build --quiet --no-default-features --features "$combo" 2>&1; then
    echo "BUILD FAILED"; fail=1; continue
  fi
  out=$(cargo test --no-default-features --features "$combo" -- --test-threads=1 2>&1)
  rc=$?
  passed=$(printf '%s\n' "$out" | grep -o 'test result: ok\. [0-9]* passed' \
           | grep -o '[0-9]*' | awk '{s+=$1} END {print s+0}')
  if [ $rc -ne 0 ]; then
    echo "TEST FAILED"
    printf '%s\n' "$out" | grep -E '^(test |---- |thread |error)' | tail -40
    fail=1
  elif [ "${passed:-0}" -lt 34 ]; then
    echo "ONLY ${passed:-0} TESTS PASSED (expected >= 34)"
    fail=1
  else
    echo "ok ($passed tests passed)"
  fi
done
echo "feature matrix done fail=$fail"
exit $fail
