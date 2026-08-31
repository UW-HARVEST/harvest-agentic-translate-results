#!/bin/bash
# Enumerate all VALID feature combinations: at most one OP feature x at most one REPEAT feature.
cd "$(dirname "$0")/translation" || exit 1
fail=0
for op in "" add sub mul; do
  for rep in "" 0 1 2 3 4 5 6 7; do
    combo=$(echo "$op $rep" | tr ' ' ',' | sed 's/^,//; s/,$//')
    out=$(cargo check --no-default-features --features "$combo" 2>&1)
    if [ $? -ne 0 ]; then echo "FAIL: [$combo]"; echo "$out" | tail -20; fail=1
    else echo "ok:   [$combo]"; fi
  done
done
# also a few conflicting (still-must-compile) combos
for combo in "add,sub" "add,sub,mul" "mul,3,5" "0,1,2,3,4,5,6,7" "add,sub,mul,0,1,2,3,4,5,6,7"; do
  out=$(cargo check --no-default-features --features "$combo" 2>&1)
  if [ $? -ne 0 ]; then echo "FAIL: [$combo]"; echo "$out" | tail -20; fail=1
  else echo "ok:   [$combo]"; fi
done
exit $fail
