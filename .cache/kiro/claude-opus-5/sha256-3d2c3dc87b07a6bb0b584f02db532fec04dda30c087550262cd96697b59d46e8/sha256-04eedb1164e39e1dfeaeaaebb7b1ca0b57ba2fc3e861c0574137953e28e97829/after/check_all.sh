#!/usr/bin/env bash
# cargo check every meaningful feature combination.
set -u
cd "$(dirname "$0")/translation"
fail=0
run() {
  local feats="$1"
  if [[ -z "$feats" ]]; then
    out=$(timeout 300 cargo check --no-default-features 2>&1)
  else
    out=$(timeout 300 cargo check --no-default-features --features "$feats" 2>&1)
  fi
  if [[ $? -ne 0 ]]; then
    echo "=== CHECK FAIL [${feats:-<none>}] ==="
    echo "$out" | grep -E '^(error|warning: unused)' | head -20
    fail=1
  fi
}
run ""
for op in add sub mul op_add op_sub op_mul; do
  for r in 0 1 2 3 4 5 6 7 repeat_0 repeat_1 repeat_2 repeat_3 repeat_4 repeat_5 repeat_6 repeat_7; do
    run "$op,$r"
  done
  run "$op"
done
for r in 0 1 2 3 4 5 6 7; do run "$r"; done
run "add,sub,mul,0,1,2,3,4,5,6,7,op_add,op_sub,op_mul,repeat_0,repeat_1,repeat_2,repeat_3,repeat_4,repeat_5,repeat_6,repeat_7"
out=$(timeout 300 cargo check --all-features 2>&1) || { echo "=== CHECK FAIL --all-features ==="; echo "$out" | grep -E '^error' | head -20; fail=1; }
[[ $fail -eq 0 ]] && echo "ALL FEATURE COMBOS CHECK OK"
exit $fail
