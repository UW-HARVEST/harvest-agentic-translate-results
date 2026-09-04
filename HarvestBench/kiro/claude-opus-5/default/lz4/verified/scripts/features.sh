#!/bin/bash
# Enumerate feature combinations declared in Cargo.toml
set -e
cd "$(dirname "$0")/.."
feats=$(cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c '
import json,sys
m=json.load(sys.stdin)
for p in m["packages"]:
    for f in p.get("features",{}):
        print(f)
')
if [ -z "$feats" ]; then
  echo "NO_FEATURES"
else
  echo "$feats"
fi
