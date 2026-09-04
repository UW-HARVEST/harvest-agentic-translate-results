#!/bin/sh
# Enumerate every feature declared in Cargo.toml and test each combination.
set -e
FEATS=$(sed -n '/^\[features\]/,/^\[/p' Cargo.toml | grep -E '^[a-zA-Z0-9_-]+ *=' | cut -d= -f1 | tr -d ' ' || true)
if [ -z "$FEATS" ]; then
  echo "Cargo.toml declares NO [features]; the only configuration is the default one."
  echo "== default =="
  cargo test --offline --release -- --test-threads=1 >/dev/null && echo "default: PASS"
  echo "== --no-default-features =="
  cargo test --offline --release --no-default-features -- --test-threads=1 >/dev/null && echo "no-default-features: PASS"
  exit 0
fi
echo "features: $FEATS"
n=0
for f in $FEATS; do n=$((n+1)); done
max=$(( (1 << n) - 1 ))
i=0
while [ $i -le $max ]; do
  combo=""; k=0
  for f in $FEATS; do
    if [ $(( (i >> k) & 1 )) -eq 1 ]; then combo="$combo,$f"; fi
    k=$((k+1))
  done
  combo=${combo#,}
  echo "== combo: [${combo}] =="
  cargo test --offline --release --no-default-features --features "$combo" -- --test-threads=1 >/dev/null \
    && echo "[$combo]: PASS" || { echo "[$combo]: FAIL"; exit 1; }
  i=$((i+1))
done
