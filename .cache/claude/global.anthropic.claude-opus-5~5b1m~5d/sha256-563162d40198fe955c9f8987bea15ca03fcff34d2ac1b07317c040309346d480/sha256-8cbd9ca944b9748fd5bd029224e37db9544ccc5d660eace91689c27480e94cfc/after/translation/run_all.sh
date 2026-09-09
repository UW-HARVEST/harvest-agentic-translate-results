#!/bin/bash
# Build both libraries and run every differential test file, for every Cargo
# feature combination (this crate declares none, so there is exactly one).
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null || exit 1
cd "$ROOT/translation" || exit 1

FEATURES=$(python3 - <<'PY'
import re
s=open('Cargo.toml').read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.S|re.M)
if not m: print('')  # no features at all -> single (default) configuration
else:
    print(' '.join(re.findall(r'^([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)))
PY
)
echo "declared features: '${FEATURES}'"

run() { echo "=== $* ==="; timeout 900 cargo test --offline --release "$@" 2>&1 | grep -E '^test |^error|test result|panicked'; }

if [ -z "$FEATURES" ]; then
  echo "--- single configuration (no [features] in Cargo.toml) ---"
  run
else
  run --no-default-features
  for f in $FEATURES; do run --no-default-features --features "$f"; done
  run --all-features
fi
