#!/usr/bin/env bash
# Phase D driver: runs the full differential suite under every buildable
# feature combination and both profiles.
set -uo pipefail
cd "$(dirname "$0")"

# Enumerate the [features] table from Cargo.toml (empty here -> default only).
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml | tr -d '"')
if [ -z "$FEATURES" ]; then
  COMBOS=("<default>" "<no-default-features>")
else
  COMBOS=("<default>" "<no-default-features>")
  for f in $FEATURES; do COMBOS+=("$f"); done
  COMBOS+=("$(echo $FEATURES | tr ' ' ',')")
fi

fail=0
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    case "$combo" in
      "<default>")             FLAGS=() ;;
      "<no-default-features>") FLAGS=(--no-default-features) ;;
      *)                       FLAGS=(--no-default-features --features "$combo") ;;
    esac
    label="profile='${profile:-dev}' features='$combo'"
    echo "=============================================================="
    echo ">>> $label"
    # Build the cdylib for this configuration first: the tests dlopen it.
    if ! timeout 600 cargo build --offline $profile "${FLAGS[@]}" >/dev/null 2>&1; then
      echo "!!! BUILD FAILED: $label"; fail=1; continue
    fi
    out=$(timeout 600 cargo test --offline $profile "${FLAGS[@]}" 2>&1)
    echo "$out" | grep -E "^test result:" || true
    if echo "$out" | grep -qE "^test result: FAILED|error(\[|:)"; then
      echo "!!! TESTS FAILED: $label"
      echo "$out" | grep -vE "^\[(INFO|WARNING|ERROR)\]" | grep -E "FAILED|panicked|assertion|error" | head -30
      fail=1
    else
      echo "OK: $label"
    fi
  done
done
echo "=============================================================="
[ $fail -eq 0 ] && echo "ALL CONFIGURATIONS PASSED" || echo "SOME CONFIGURATIONS FAILED"
exit $fail
