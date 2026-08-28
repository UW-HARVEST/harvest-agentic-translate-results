#!/usr/bin/env bash
# Phase A/D helper: `cargo check` every feature combination (valid ones plus the
# conflicting ones, which must still compile).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/translation"

fail=0
run() { # $1 = feature list (may be empty)
  local feats="$1" out
  if [ -z "$feats" ]; then
    out=$(timeout 300 cargo check --offline --no-default-features --all-targets 2>&1)
  else
    out=$(timeout 300 cargo check --offline --no-default-features --features "$feats" --all-targets 2>&1)
  fi
  if [ $? -ne 0 ]; then
    echo "FAIL [${feats:-<none>}]"
    echo "$out" | tail -n 20
    fail=1
  elif echo "$out" | grep -q "^warning"; then
    echo "WARN [${feats:-<none>}]"
    echo "$out" | grep -A3 "^warning" | head -n 12
  else
    echo "ok   [${feats:-<none>}]"
  fi
}

echo "=== valid combinations (<=1 OP, <=1 REPEAT) ==="
while IFS='|' read -r feats op rep; do
  run "$feats"
done < <("$ROOT/scripts/combos.sh")

echo "=== conflicting combinations (must still compile) ==="
for feats in add,sub add,mul sub,mul add,sub,mul 0,1,2,3,4,5,6,7 add,sub,mul,0,1,2,3,4,5,6,7 mul,7 sub,0; do
  run "$feats"
done

if [ "$fail" -ne 0 ]; then echo "SOME COMBINATIONS FAILED"; exit 1; fi
echo "ALL COMBINATIONS COMPILE CLEAN"
