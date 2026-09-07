#!/bin/bash
# cargo check for every valid feature combination (one OP x one REPEAT),
# plus the degenerate combos (none / all) which must also compile.
set -uo pipefail
cd "$(dirname "$0")/translation"
CMD="${1:-check}"
fail=0
combos=()
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    combos+=("$op,$rep")
  done
done
combos+=("" "add,sub,mul,0,1,2,3,4,5,6,7")

for c in "${combos[@]}"; do
  if [ -z "$c" ]; then
    out=$(cargo "$CMD" --offline --all-targets --no-default-features 2>&1)
  else
    out=$(cargo "$CMD" --offline --all-targets --no-default-features --features "$c" 2>&1)
  fi
  if [ $? -ne 0 ]; then
    echo "FAIL [${c:-<none>}]"
    echo "$out" | grep -E "^(error|warning: unused)" | head -20
    fail=1
  else
    w=$(echo "$out" | grep -c "^warning")
    echo "ok   [${c:-<none>}] warnings=$w"
  fi
done
exit $fail
