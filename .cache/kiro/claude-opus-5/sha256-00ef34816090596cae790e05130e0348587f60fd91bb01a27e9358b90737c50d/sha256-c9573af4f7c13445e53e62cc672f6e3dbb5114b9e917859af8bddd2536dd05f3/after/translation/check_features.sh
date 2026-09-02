#!/usr/bin/env bash
# Phase D: run the full differential suite under EVERY feature combination.
#
# Features are extracted from Cargo.toml rather than hardcoded, so this stays
# correct if a [features] table is ever added.
set -u
cd "$(dirname "$0")" || exit 1

# Extract feature names from the [features] section of Cargo.toml.
features=$(awk '
  /^\[features\]/ { inside=1; next }
  /^\[/           { inside=0 }
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    sub(/[[:space:]]*=.*/, "", $0); print $0
  }
' Cargo.toml | grep -v '^default$')

echo "declared non-default features: [${features:-<none>}]"


fail=0
check() {
  local label="$1"; shift
  printf '%-52s' "$label"
  if timeout 600 cargo test "$@" >/tmp/ft.log 2>&1; then
    # Sum the per-binary counts; the lib-unittest binary contributes 0.
    total=$(grep -oE '[0-9]+ passed' /tmp/ft.log | awk '{s+=$1} END {print s+0}')
    if [ "$total" -lt 34 ]; then
      echo "FAIL (only $total tests ran; expected the full differential suite)"
      fail=1
    else
      echo "PASS ($total tests passed)"
    fi
  else
    echo "FAIL"
    tail -25 /tmp/ft.log
    fail=1
  fi
}

# Always: the default configuration, and the no-features configuration.
check "default features" 
check "--no-default-features" --no-default-features

if [ -n "$features" ]; then
  # Full power set of the declared features.
  list=($features)
  n=${#list[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then
        combo="${combo:+$combo,}${list[$i]}"
      fi
    done
    check "--no-default-features --features $combo" \
      --no-default-features --features "$combo"
  done
  check "--all-features" --all-features
fi

if [ $fail -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASS"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit $fail
