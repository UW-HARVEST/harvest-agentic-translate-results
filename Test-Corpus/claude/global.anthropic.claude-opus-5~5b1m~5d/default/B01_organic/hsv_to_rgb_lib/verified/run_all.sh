#!/usr/bin/env bash
# Phase D driver: run the full differential suite under every feature
# combination and both cargo profiles.
set -uo pipefail
cd "$(dirname "$0")"

# Enumerate feature combinations from Cargo.toml (there are none declared, so
# the set is just the three equivalent no-feature invocations).
FEATS=$(python3 - <<'PY'
import re,itertools,sys
s=open('Cargo.toml').read()
m=re.search(r'^\[features\](.*?)(^\[|\Z)', s, re.M|re.S)
names=[]
if m:
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if '=' in line:
            n=line.split('=')[0].strip()
            if n!='default': names.append(n)
print(len(names))
PY
)
echo "declared non-default features: $FEATS"

COMBOS=("" "--no-default-features" "--all-features")
if [ "$FEATS" -gt 0 ]; then
  echo "NOTE: features exist; expand COMBOS to the full powerset." >&2
fi

FAIL=0
for prof in release ""; do
  for combo in "${COMBOS[@]}"; do
    label="profile=${prof:-dev} features=${combo:-default}"
    echo "=============================================================="
    echo ">>> $label"
    PROF_FLAG=""
    [ -n "$prof" ] && PROF_FLAG="--release"
    # Forward the feature flags to the in-test cdylib build too, so the .so
    # under test is built with the SAME configuration as the harness.
    HARNESS_CARGO_ARGS="$combo" timeout 600 cargo test $PROF_FLAG --offline $combo 2>&1 \
      | grep -E '^test result|^error|panicked|FAILED' 
    rc=${PIPESTATUS[0]}
    [ "$rc" -ne 0 ] && { echo "!!! FAILED: $label"; FAIL=1; }
  done
done
echo "=============================================================="
[ "$FAIL" -eq 0 ] && echo "ALL COMBOS: PASS" || { echo "SOME COMBOS FAILED"; exit 1; }
