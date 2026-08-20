#!/bin/bash
# cargo check every valid feature combination (canonical numeric + repeat_N alias
# forms, plus the "unspecified" forms that must fall back to the C defaults).
cd "$(dirname "$0")/.." || exit 1
fail=0
run() {
  local desc="$1"; shift
  if out=$(timeout 300 cargo check --offline --no-default-features "$@" 2>&1); then
    echo "OK    $desc"
  else
    echo "FAIL  $desc"
    echo "$out" | tail -20
    fail=1
  fi
}
for op in add sub mul; do
  for r in 0 1 2 3 4 5 6 7; do
    run "features=$op,$r"        --features "$op,$r"
    run "features=$op,repeat_$r" --features "$op,repeat_$r"
  done
  run "features=$op (REPEAT default)" --features "$op"
done
for r in 0 1 2 3 4 5 6 7; do
  run "features=$r (OP default)" --features "$r"
done
run "no features at all"
run "default features" --features default
echo "---"
[ $fail -eq 0 ] && echo "ALL FEATURE COMBOS CHECK OK" || echo "SOME COMBOS FAILED"
exit $fail
