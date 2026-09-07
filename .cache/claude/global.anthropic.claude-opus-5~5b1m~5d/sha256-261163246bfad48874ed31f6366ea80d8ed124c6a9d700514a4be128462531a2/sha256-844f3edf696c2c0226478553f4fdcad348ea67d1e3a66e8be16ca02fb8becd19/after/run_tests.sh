#!/bin/bash
# Phases B + C for EVERY feature combination.
#
#   ./run_tests.sh            # all 24 (OP x REPEAT) combos + 2 degenerate ones
#   ./run_tests.sh add,5      # a single combo
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation"

if [ $# -gt 0 ]; then
  combos=("$@")
else
  combos=()
  for op in add sub mul; do
    for rep in 0 1 2 3 4 5 6 7; do combos+=("$op,$rep"); done
  done
  # degenerate combos: no feature at all (C's `#ifndef OP/REPEAT` defaults) and
  # every feature at once (resolved by mdconfig.rs's documented priority).
  combos+=("__none__" "add,sub,mul,0,1,2,3,4,5,6,7")
fi

pass=0; fail=0
for c in "${combos[@]}"; do
  if [ "$c" = "__none__" ]; then
    feat=()
  else
    feat=(--features "$c")
  fi
  log="$ROOT/testlog_${c//,/_}.log"
  # Always rebuild the Rust .so / driver for THIS combo and point the tests at
  # the fresh artifacts, so a stale rustbuild/ copy can never be tested.
  if ! timeout 600 cargo build --release --offline --no-default-features "${feat[@]}" \
        -q > "$log" 2>&1; then
    echo "FAIL [$c] build error (see $log)"; fail=$((fail+1)); continue
  fi
  export DIFF_RUST_SO="$ROOT/translation/target/release/libdriver.so"
  export DIFF_RUST_EXE="$ROOT/translation/target/release/driver"
  if timeout 600 cargo test --release --offline --no-default-features "${feat[@]}" \
        -- --test-threads=1 > "$log" 2>&1; then
    n=$(grep -hoE '[0-9]+ passed' "$log" | awk '{s+=$1} END{print s}')
    echo "PASS [$c] ${n} assertions/tests"
    pass=$((pass+1))
    rm -f "$log"
  else
    echo "FAIL [$c]  (see $log)"
    grep -E "^(test .*FAILED|failures:|assertion|thread)" "$log" | head -20
    fail=$((fail+1))
  fi
done
echo "-----"
echo "combos passed=$pass failed=$fail"
exit $((fail > 0))
