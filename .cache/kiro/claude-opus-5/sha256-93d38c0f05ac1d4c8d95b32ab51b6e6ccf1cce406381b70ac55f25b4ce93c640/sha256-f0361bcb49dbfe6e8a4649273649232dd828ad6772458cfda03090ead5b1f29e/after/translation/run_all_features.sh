#!/usr/bin/env bash
# Phase D automation: enumerate every feature combination declared in
# Cargo.toml and run the full differential suite for each one.
set -uo pipefail
cd "$(dirname "$0")"

# Extract feature names from the [features] section (excluding "default").
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/            {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

echo "declared features: [${FEATURES:-<none>}]"

run_combo() {
  local label="$1"; shift
  echo "=============================================================="
  echo "### combo: $label"
  echo "=============================================================="
  timeout 600 cargo check "$@" 2>&1 | tail -3 || return 1
  timeout 600 cargo build --release "$@" 2>&1 | tail -2 || return 1
  timeout 600 cargo test --release "$@" 2>&1 | grep -E "^running|^test result|FAILED" || return 1
}

fail=0
run_combo "default"                            || fail=1
run_combo "no-default-features" --no-default-features || fail=1
run_combo "all-features" --all-features        || fail=1

# Powerset of the declared (non-default) features, if any exist.
if [ -n "$FEATURES" ]; then
  mapfile -t F <<< "$FEATURES"
  n=${#F[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${F[i]}"; fi
    done
    run_combo "$combo" --no-default-features --features "$combo" || fail=1
  done
fi

echo "=============================================================="
if [ "$fail" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATION FAILED"; fi
exit "$fail"
