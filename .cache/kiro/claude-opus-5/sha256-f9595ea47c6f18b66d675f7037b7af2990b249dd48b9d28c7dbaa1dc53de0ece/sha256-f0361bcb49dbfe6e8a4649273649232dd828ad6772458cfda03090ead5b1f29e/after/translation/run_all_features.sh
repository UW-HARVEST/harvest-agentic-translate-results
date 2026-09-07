#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination declared in Cargo.toml and
# run cargo check + the full differential suite under each one.
set -uo pipefail
cd "$(dirname "$0")"

# Mechanically extract feature names from [features] (excluding "default").
FEATS=$(python3 - <<'PY'
import re
txt=open("Cargo.toml").read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M|re.S)
if not m:
    print("")
else:
    names=[]
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if not line or '=' not in line: continue
        n=line.split('=')[0].strip().strip('"')
        if n!="default": names.append(n)
    print(" ".join(names))
PY
)

COMBOS=()
if [ -z "$FEATS" ]; then
  echo "Cargo.toml declares NO [features] -> exactly 1 configuration."
  COMBOS+=("<default>" "--no-default-features")
else
  read -ra ARR <<< "$FEATS"
  n=${#ARR[@]}
  COMBOS+=("<default>")
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0;i<n;i++)); do (( mask & (1<<i) )) && sel+=("${ARR[i]}"); done
    if [ ${#sel[@]} -eq 0 ]; then COMBOS+=("--no-default-features")
    else COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")"); fi
  done
fi

rc=0
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "<default>" ]; then args=(); label="(default features)"
  else read -ra args <<< "$combo"; label="$combo"; fi
  echo "=============================================================="
  echo "FEATURE COMBO: $label"
  echo "=============================================================="
  if ! timeout 600 cargo check "${args[@]}" >/tmp/fc_check.log 2>&1; then
    echo "  cargo check FAILED"; tail -20 /tmp/fc_check.log; rc=1; continue
  fi
  echo "  cargo check ok"
  if ! timeout 600 cargo test --release "${args[@]}" -- --test-threads=1 >/tmp/fc_test.log 2>&1; then
    echo "  cargo test FAILED"; grep -E '^test result:|DIVERGENCE' /tmp/fc_test.log | head -20; rc=1; continue
  fi
  grep -E '^test result:' /tmp/fc_test.log | sed 's/^/  /'
done
echo "=============================================================="
[ $rc -eq 0 ] && echo "ALL FEATURE COMBINATIONS PASSED" || echo "SOME COMBINATIONS FAILED"
exit $rc
