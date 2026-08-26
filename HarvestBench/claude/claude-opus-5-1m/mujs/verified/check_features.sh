#!/bin/sh
# Phase D: enumerate every valid feature combination from Cargo.toml and run
# `cargo check` for each. mujs has NO [features] section and no optional
# dependencies, so the powerset is a single element (the empty set); the three
# invocations below are the three spellings of it and MUST all succeed.
set -e
cd "$(dirname "$0")"

echo "=== declared features ==="
python3 - <<'PY'
import re, itertools
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', s, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            feats.append(line.split('=')[0].strip())
print("features:", feats if feats else "(none)")
combos = [list(c) for r in range(len(feats)+1) for c in itertools.combinations(feats, r)]
with open('.feature_combos', 'w') as f:
    for c in combos:
        f.write(','.join(c) + '\n')
print("combinations:", len(combos))
PY

while IFS= read -r combo; do
    if [ -z "$combo" ]; then
        echo "--- combo <empty>"
        cargo check --no-default-features
        cargo build --no-default-features
    else
        echo "--- combo $combo"
        cargo check --no-default-features --features "$combo"
        cargo build --no-default-features --features "$combo"
    fi
done < .feature_combos

echo "--- also: default features, and --all-features"
cargo check
cargo check --all-features
cargo build --all-features
rm -f .feature_combos
echo "ALL FEATURE COMBINATIONS OK"
