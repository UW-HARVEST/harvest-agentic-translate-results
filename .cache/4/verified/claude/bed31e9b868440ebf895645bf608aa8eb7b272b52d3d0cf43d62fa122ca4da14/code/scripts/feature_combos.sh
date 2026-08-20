#!/usr/bin/env bash
# Print every valid feature combination (one per line; empty first line == the
# default/no-feature build). Derived mechanically from Cargo.toml's [features].
set -eu
cd "$(dirname "$0")/.."

python3 - <<'EOF'
import itertools, re, sys

text = open("Cargo.toml").read()
m = re.search(r"^\[features\]\s*$(.*?)(^\[|\Z)", text, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split("#")[0].strip()
        if not line or "=" not in line:
            continue
        name = line.split("=")[0].strip().strip('"')
        if name and name != "default":
            feats.append(name)

# The empty set (== --no-default-features) is always a valid combination.
combos = [""]
for k in range(1, len(feats) + 1):
    for c in itertools.combinations(feats, k):
        combos.append(",".join(c))
print("\n".join(combos))
EOF
