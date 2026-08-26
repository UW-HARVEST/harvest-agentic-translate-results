#!/usr/bin/env bash
# Phase A/D: enumerate every valid build configuration and check/test each one.
#
# `Cargo.toml` has no [features] table and `c_src/` has no #ifdef / cmake
# option, so the enumeration below is the COMPLETE cross product: the empty
# feature set, spelled three equivalent ways, x {dev, release} profile.
set -u
cd "$(dirname "$0")"

feats=$(sed -n '/^\[features\]/,/^\[/p' Cargo.toml | grep -E '^[a-zA-Z0-9_-]+ *=' | cut -d= -f1 | tr -d ' ')
if [ -n "$feats" ]; then
  echo "!! Cargo.toml declares features: $feats -- extend this script" >&2
  exit 1
fi
echo "== Cargo.toml declares NO features: the only combination is the empty set =="

rc=0
for flags in "--no-default-features" "--all-features" ""; do
  for prof in "" "--release"; do
    label="cargo check ${flags:-<default>} ${prof:-<dev>}"
    printf '%-52s ' "$label"
    if timeout 300 cargo check --offline --all-targets $flags $prof > "${TMPDIR:-/tmp}/check.log" 2>&1; then
      echo OK
    else
      echo FAIL; tail -30 "${TMPDIR:-/tmp}/check.log"; rc=1
    fi
  done
done

for flags in "--no-default-features" "--all-features" ""; do
  for prof in "" "--release"; do
    label="cargo test  ${flags:-<default>} ${prof:-<dev>}"
    echo "=============================================================="
    echo "== $label"
    echo "=============================================================="
    if ! timeout 600 cargo test --offline --test differential $flags $prof; then
      echo "!! FAILED: $label"; rc=1
    fi
  done
done
exit $rc
