#!/bin/bash
# Runs the full differential suite (Phases B, C, D) for EVERY valid feature
# combination derived from Cargo.toml [features] / c_src/CMakeLists.txt.
#
#   OP     -> add | sub | mul          (3)
#   REPEAT -> repeat_0 .. repeat_7     (8)   + the bare aliases "0".."7"
#            plus the "no feature at all" build (the #ifndef fallbacks).
#
# Usage: ./run_all_features.sh [--quick]
#   --quick  only the 24 canonical combos (skips the 24 alias combos)
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation"

QUICK=0
[ "${1:-}" = "--quick" ] && QUICK=1

pass=0; fail=0; failed_combos=()

run_combo() {
  local desc="$1"; shift
  local out
  out=$(timeout 600 cargo test --no-default-features "$@" \
          -- --test-threads=1 2>&1)
  local rc=$?
  local summary
  summary=$(echo "$out" | grep -E '^test result:' | tr '\n' ' ')
  if [ $rc -ne 0 ]; then
    fail=$((fail+1)); failed_combos+=("$desc")
    echo "FAIL  $desc"
    echo "$out" | grep -E 'panicked|^    [a-z0-9_]+$|error\[|^error' | head -20
  else
    pass=$((pass+1))
    echo "ok    $desc   [$summary]"
  fi
}

echo "=== no features at all (C: no -DOP/-DREPEAT) ==="
run_combo "<none>"

echo
echo "=== 24 canonical combos: OP x repeat_N ==="
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    run_combo "$op,repeat_$rep" --features "$op,repeat_$rep"
  done
done

if [ $QUICK -eq 0 ]; then
  echo
  echo '=== 24 alias combos: OP x bare CMake value "N" ==='
  for op in add sub mul; do
    for rep in 0 1 2 3 4 5 6 7; do
      run_combo "$op,$rep (alias)" --features "$op,$rep"
    done
  done
fi

echo
echo "=== default feature set (add, repeat_5) ==="
out=$(timeout 600 cargo test -- --test-threads=1 2>&1)
if [ $? -ne 0 ]; then
  fail=$((fail+1)); failed_combos+=("default")
  echo "FAIL  default"; echo "$out" | grep -E 'panicked|^    [a-z0-9_]+$' | head -20
else
  pass=$((pass+1))
  echo "ok    default   [$(echo "$out" | grep -E '^test result:' | tr '\n' ' ')]"
fi

echo
echo "================ SUMMARY ================"
echo "combinations passed: $pass"
echo "combinations failed: $fail"
if [ $fail -ne 0 ]; then
  printf 'failed: %s\n' "${failed_combos[@]}"
  exit 1
fi
