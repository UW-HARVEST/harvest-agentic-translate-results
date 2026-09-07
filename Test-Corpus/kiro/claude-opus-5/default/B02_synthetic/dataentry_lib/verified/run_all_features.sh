#!/usr/bin/env bash
# Runs the full differential suite under EVERY cargo feature combination.
# Feature names are extracted from Cargo.toml rather than hard-coded.
set -uo pipefail
cd "$(dirname "$0")"

# Extract feature names from the [features] section, if any.
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); if (a[1] != "default") print a[1] }
' Cargo.toml)

echo "== declared features: [${FEATURES//$'\n'/ }]"

run() { # run <label> <extra cargo args...>
  local label="$1"; shift
  echo "---- $label ----"
  if timeout 600 cargo test "$@" 2>&1 | grep -E 'test result:|FAILED|divergence|^error'; then :; fi
}

run "debug profile, default features"
run "release profile, default features" --release
run "release profile, --no-default-features" --release --no-default-features
run "debug profile, --no-default-features" --no-default-features

if [ -n "$FEATURES" ]; then
  # every non-empty subset of the declared features
  mapfile -t F <<< "$FEATURES"
  n=${#F[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="$combo,${F[i]}"; fi
    done
    combo="${combo#,}"
    run "--no-default-features --features $combo" --release --no-default-features --features "$combo"
  done
else
  echo "== no [features] table in Cargo.toml: default and --no-default-features"
  echo "== are the only configurations, and both were run above."
fi
