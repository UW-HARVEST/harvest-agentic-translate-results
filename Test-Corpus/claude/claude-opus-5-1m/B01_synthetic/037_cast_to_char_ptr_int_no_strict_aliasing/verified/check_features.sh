#!/usr/bin/env bash
# Enumerate every feature declared in Cargo.toml and cargo check every subset.
set -u
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{print $1}' Cargo.toml)
echo "declared features: [${FEATURES}]"
COMBOS=("")   # the empty combination == --no-default-features
if [ -n "${FEATURES}" ]; then
  ARR=(${FEATURES})
  N=${#ARR[@]}
  for ((mask=1; mask<(1<<N); mask++)); do
    c=""
    for ((i=0;i<N;i++)); do
      if (( mask & (1<<i) )); then c="${c:+$c,}${ARR[$i]}"; fi
    done
    COMBOS+=("$c")
  done
fi
rc=0
for c in "${COMBOS[@]}"; do
  echo "=== cargo check --no-default-features --features '${c}' ==="
  timeout 600 cargo check --offline --all-targets --no-default-features ${c:+--features "$c"} 2>&1 | tail -5
  s=${PIPESTATUS[0]}; [ "$s" -ne 0 ] && rc=1
done
echo "=== cargo check (default features) ==="
timeout 600 cargo check --offline --all-targets 2>&1 | tail -5 || rc=1
echo "=== cargo check --all-features ==="
timeout 600 cargo check --offline --all-targets --all-features 2>&1 | tail -5 || rc=1
echo "OVERALL_RC=$rc"
