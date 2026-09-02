#!/usr/bin/env bash
# Phase D: enumerate the cargo feature power set from Cargo.toml and `cargo check`
# every combination. Automated rather than hand-repeated, per the task tip.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

# Extract feature names from the [features] section, if any.
FEATURES=$(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
print(' '.join(names))
PY
)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features]; the only build configuration is the default."
  echo "-> cargo check (default)"
  timeout 600 cargo check --quiet
  echo "-> cargo check --no-default-features"
  timeout 600 cargo check --quiet --no-default-features
  echo "-> cargo check --all-features"
  timeout 600 cargo check --quiet --all-features
  echo "OK: all (1) feature configuration checks pass."
  exit 0
fi

read -ra ARR <<< "$FEATURES"
N=${#ARR[@]}
echo "Found $N features: ${ARR[*]}"
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if (((mask >> i) & 1)); then combo="$combo,${ARR[i]}"; fi
  done
  combo="${combo#,}"
  echo "-> --no-default-features --features '$combo'"
  timeout 600 cargo check --quiet --no-default-features --features "$combo"
  echo "-> test  --no-default-features --features '$combo'"
  timeout 600 cargo test --quiet --release --no-default-features --features "$combo"
done
echo "OK: all $((1 << N)) feature combinations pass."
