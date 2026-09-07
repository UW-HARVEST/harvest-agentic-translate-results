#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under EVERY feature
# combination declared in Cargo.toml, in both debug and release profiles.
set -uo pipefail

cd "$(dirname "$0")"

# Mechanically extract the feature list from Cargo.toml.
mapfile -t FEATURES < <(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml)

# All combinations of the declared features (there are few enough to enumerate).
COMBOS=("")
for f in "${FEATURES[@]}"; do
    new=()
    for c in "${COMBOS[@]}"; do
        new+=("$c")
        if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    COMBOS=("${new[@]}")
done

rc=0
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="features='${combo:-<none>}' profile='${profile:-debug}'"
    echo "=============================================================="
    echo "== cargo test --no-default-features --features '${combo}' ${profile}"
    echo "=============================================================="
    if [ -z "$combo" ]; then
        timeout 600 cargo test --offline --no-default-features $profile 2>&1 | tail -25
    else
        timeout 600 cargo test --offline --no-default-features --features "$combo" $profile 2>&1 | tail -25
    fi
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then
        echo "FAILED: $label"
        rc=1
    else
        echo "PASSED: $label"
    fi
  done
done

echo
if [ "$rc" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit "$rc"
