#!/bin/bash
# cargo check every valid feature combination
R="$(cd "$(dirname "$0")" && pwd)"
cd "$R/translation"
BACKENDS="haraka sha2 shake shake256 blake"
THASHES="robust simple"
SECPARS="128s 128f 192s 192f 256s 256f"
fail=0
for b in $BACKENDS; do for t in $THASHES; do for s in $SECPARS; do
  combo="$b,$t,$s"
  out=$(cargo check --no-default-features --features "$combo" 2>&1)
  if echo "$out" | grep -q "^error"; then
    echo "FAIL $combo"
    echo "$out" | grep -E "^(error|warning: unused)" | head -10
    fail=1
  else
    echo "ok   $combo"
  fi
done; done; done
exit $fail
