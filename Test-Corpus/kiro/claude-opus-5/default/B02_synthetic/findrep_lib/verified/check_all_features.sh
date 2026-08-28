#!/usr/bin/env bash
# Enumerate every feature combination declared in Cargo.toml and run
# `cargo check` + `cargo test` for each one.
set -uo pipefail
cd "$(dirname "$0")"

# Extract feature names from the [features] table (ignore the implicit
# "default" key, which is not itself a selectable feature).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f=1; next }
    /^\[/           { in_f=0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "Declared features (${n}): ${FEATURES[*]:-<none>}"

combos=()
if [ "$n" -eq 0 ]; then
  combos+=("")            # only the (empty) default configuration exists
else
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    combos+=("$combo")
  done
fi

echo "Combinations to verify: ${#combos[@]}"
fail=0
for combo in "${combos[@]}"; do
  label="${combo:-<no features>}"
  echo "=============================================================="
  echo "### $label"
  for stage in check test; do
    if [ -z "$combo" ]; then
      timeout 600 cargo "$stage" --no-default-features > /tmp/fc.log 2>&1
    else
      timeout 600 cargo "$stage" --no-default-features --features "$combo" > /tmp/fc.log 2>&1
    fi
    rc=$?
    if [ $rc -ne 0 ]; then
      echo "FAIL cargo $stage ($label) rc=$rc"
      tail -40 /tmp/fc.log
      fail=1
    else
      echo "ok   cargo $stage ($label)"
    fi
  done
done

echo "=============================================================="
[ $fail -eq 0 ] && echo "ALL FEATURE COMBINATIONS PASSED" || echo "FAILURES PRESENT"
exit $fail
