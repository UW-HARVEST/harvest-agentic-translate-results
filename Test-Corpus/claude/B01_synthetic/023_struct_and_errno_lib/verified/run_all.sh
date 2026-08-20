#!/usr/bin/env bash
# Runs cargo check + the full differential test suite for EVERY valid feature
# combination.  Cargo.toml has no [features] section, so the complete set of
# combinations is the single empty one -- enumerated mechanically below rather
# than hard-coded.
set -u
COMBOS=$(python3 - <<'PY'
import itertools, re, sys
txt = open("Cargo.toml").read()
m = re.search(r"^\[features\]\s*$(.*?)(^\[|\Z)", txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            feats.append(line.split('=')[0].strip())
combos = []
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        combos.append(",".join(c))
print("\n".join(combos))
PY
)
rc=0
while IFS= read -r combo; do
  label="${combo:-<none/default>}"
  echo "=================================================================="
  echo "### feature combination: $label"
  echo "=================================================================="
  timeout 600 cargo check --offline --no-default-features --features "$combo" 2>&1 | tail -5 || rc=1
  timeout 600 cargo test  --offline --no-default-features --features "$combo" 2>&1 \
    | grep -E "^(running|test result)|FAILED|panicked|^error" || rc=1
  echo "### release profile (panic=abort, no debug-assertions)"
  timeout 600 cargo test --offline --release --no-default-features --features "$combo" 2>&1 \
    | grep -E "^(running|test result)|FAILED|panicked|^error" || rc=1
done <<< "$COMBOS"
exit $rc
