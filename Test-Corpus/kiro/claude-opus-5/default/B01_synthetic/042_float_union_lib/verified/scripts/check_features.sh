#!/usr/bin/env bash
# Enumerates every Cargo feature combination from `cargo metadata` (rather than
# assuming there are none) and runs `cargo check` for each.
set -euo pipefail
cd "$(dirname "$0")/.."

# `cargo metadata` emits `"features":{...}` for the package; extract the keys
# without needing jq.
raw=$(cargo metadata --no-deps --format-version 1)
feat_block=$(printf '%s' "$raw" | sed -n 's/.*"features":{\([^}]*\)}.*/\1/p')
if [ -z "$feat_block" ]; then
  names=""
else
  names=$(printf '%s' "$feat_block" | grep -o '"[^"]*":' | tr -d '":' || true)
fi

echo "declared features: [${names//$'\n'/ }]"

if [ -z "$names" ]; then
  echo "No features declared -> the only combination is the default one."
  echo "== cargo check (default) =="
  timeout 300 cargo check --all-targets
  echo "== cargo check --no-default-features =="
  timeout 300 cargo check --no-default-features --all-targets
  echo "OK: 1 feature combination checked (default == empty)."
  exit 0
fi

# Non-empty feature set: check the full power set.
mapfile -t arr <<<"$names"
n=${#arr[@]}
total=$((1 << n))
for ((mask = 0; mask < total; mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then combo="$combo,${arr[i]}"; fi
  done
  combo=${combo#,}
  echo "== cargo check --no-default-features --features '$combo' =="
  timeout 300 cargo check --no-default-features --features "$combo" --all-targets
done
echo "OK: $total feature combinations checked."
