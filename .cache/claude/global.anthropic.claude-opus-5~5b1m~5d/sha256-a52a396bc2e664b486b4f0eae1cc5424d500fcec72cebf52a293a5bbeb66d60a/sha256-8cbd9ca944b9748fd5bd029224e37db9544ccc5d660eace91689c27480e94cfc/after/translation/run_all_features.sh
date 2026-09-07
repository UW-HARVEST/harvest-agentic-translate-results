#!/bin/bash
# Phase D: run the full differential suite under every feature combination.
set -u
cd "$(dirname "$0")"
# Enumerate features from Cargo.toml ([features] section keys, minus "default").
FEATS=$(python3 - <<'PY'
import re
s=open("Cargo.toml").read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M|re.S)
keys=[]
if m:
    for line in m.group(1).splitlines():
        line=line.strip()
        if line and not line.startswith('#') and '=' in line:
            k=line.split('=')[0].strip()
            if k!='default': keys.append(k)
print(' '.join(keys))
PY
)
echo "features found: '${FEATS}'"

# Build the combination list: default, no-default-features, and (if any features
# exist) every non-empty subset of them.
declare -a COMBOS
COMBOS+=("DEFAULT")
COMBOS+=("NODEFAULT")
if [ -n "${FEATS}" ]; then
  arr=($FEATS); n=${#arr[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${arr[$i]}"; fi
    done
    COMBOS+=("FEAT:${combo}")
  done
fi

fail=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT)    args=() ;;
    NODEFAULT)  args=(--no-default-features) ;;
    FEAT:*)     args=(--no-default-features --features "${combo#FEAT:}") ;;
  esac
  echo "=============== combination: $combo (${args[*]:-<none>}) ==============="
  # the .so under test must be rebuilt for this combination first
  cargo build --release --offline "${args[@]}" >/dev/null 2>&1 || { echo "BUILD FAILED"; fail=1; continue; }
  cargo check --offline "${args[@]}" 2>&1 | grep -E '^(error|warning: unused)' && fail=1
  if timeout 600 cargo test --offline "${args[@]}" 2>&1 | tee /dev/stderr | grep -qE 'test result: FAILED|[1-9][0-9]* failed'; then
    echo "TESTS FAILED for $combo"; fail=1
  else
    echo "OK: $combo"
  fi
done
exit $fail
