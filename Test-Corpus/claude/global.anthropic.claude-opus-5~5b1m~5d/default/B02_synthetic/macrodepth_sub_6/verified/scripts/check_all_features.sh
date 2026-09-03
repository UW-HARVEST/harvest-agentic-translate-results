#!/usr/bin/env bash
# Runs `cargo check` for every enumerated feature combination.
# Usage: bash scripts/check_all_features.sh   (from the crate root)
set -u
cd "$(dirname "$0")/.."

ops=("" "add" "sub" "mul" "add,sub" "add,mul" "sub,mul" "add,sub,mul")
reps=("" "0" "1" "2" "3" "4" "5" "6" "7" "0,7" "3,5" "0,1,2,3,4,5,6,7")

fail=0
n=0
for o in "${ops[@]}"; do
  for r in "${reps[@]}"; do
    if [ -z "$o" ] && [ -z "$r" ]; then combo=""
    elif [ -z "$o" ]; then combo="$r"
    elif [ -z "$r" ]; then combo="$o"
    else combo="$o,$r"; fi
    n=$((n + 1))
    if ! out=$(cargo check --quiet --all-targets --no-default-features --features "$combo" 2>&1); then
      echo "FAIL [$combo]"
      printf '%s\n' "$out" | head -30
      fail=1
    fi
  done
done
echo "checked $n combinations, fail=$fail"
exit $fail
