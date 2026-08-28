#!/usr/bin/env python3
"""Print every non-`default` feature declared in Cargo.toml, one per line."""
import re

txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
print("\n".join(feats))
