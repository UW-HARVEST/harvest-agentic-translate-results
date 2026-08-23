#!/bin/bash
# Enumerate every valid feature combination of this crate and check each one.
#
# `Cargo.toml` has no [features] section, so the power set of the feature set is
# the single empty combination.  This script derives that mechanically rather
# than assuming it, and runs the whole differential suite for each combination.
set -u
cd "$(dirname "$0")/.."
mkdir -p target

feats=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).split('\n'):
        line = line.split('#')[0].strip()
        mm = re.match(r'^([A-Za-z0-9_.-]+)\s*=', line)
        if mm and mm.group(1) != 'default':
            names.append(mm.group(1))
print(' '.join(names))
PY
)

echo "declared features: [${feats}]"
python3 - "$feats" <<'PY' > target/feature_combos.txt
import itertools, sys
names = sys.argv[1].split()
for r in range(len(names) + 1):
    for c in itertools.combinations(names, r):
        print(','.join(c))
PY
n=$(wc -l < target/feature_combos.txt)
echo "valid feature combinations: $n"

rc=0
while IFS= read -r combo; do
    echo "===================================================================="
    echo "=== combination: '${combo:-(empty)}'"
    echo "===================================================================="
    cargo check  --offline --no-default-features --features "$combo" || rc=1
    cargo build  --offline --release --no-default-features --features "$combo" || rc=1
    ./check.sh --no-default-features --features "$combo" || rc=1
done < target/feature_combos.txt
exit $rc
