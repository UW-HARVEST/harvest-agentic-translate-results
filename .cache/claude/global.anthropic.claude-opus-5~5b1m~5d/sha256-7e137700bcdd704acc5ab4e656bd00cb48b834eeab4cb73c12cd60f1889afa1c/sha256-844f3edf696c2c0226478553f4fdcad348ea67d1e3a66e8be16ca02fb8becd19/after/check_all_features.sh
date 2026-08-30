#!/usr/bin/env bash
# Phase A step 2 / Phase D: `cargo check` EVERY valid feature combination.
#
# The crate exposes 11 features (translation/Cargo.toml):
#   OP     -> add | sub | mul          (mirrors CMake -DOP=)
#   REPEAT -> 0 1 2 3 4 5 6 7          (mirrors CMake -DREPEAT=)
# Cargo features are additive and cannot be made mutually exclusive, so all
# 2^11 = 2048 subsets are reachable by a consumer and all of them must compile.
# This script checks all 2048 by default (pass `quick` for the 24 canonical
# CMake configurations plus the documented conflict/default cases only).
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT/translation"
export CARGO_NET_OFFLINE=true

FEATURES=(add sub mul 0 1 2 3 4 5 6 7)
MODE="${1:-full}"

combos=()
if [[ "$MODE" == "quick" ]]; then
  combos+=("")                                  # no features -> add / 5 defaults
  for op in add sub mul; do combos+=("$op"); done
  for r in 0 1 2 3 4 5 6 7; do combos+=("$r"); done
  for op in add sub mul; do for r in 0 1 2 3 4 5 6 7; do combos+=("$op,$r"); done; done
  combos+=("add,sub" "add,mul" "sub,mul" "add,sub,mul")
  combos+=("2,5" "0,7" "0,1,2,3,4,5,6,7")
  combos+=("add,sub,mul,0,1,2,3,4,5,6,7")
else
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && sel+=("${FEATURES[b]}")
    done
    combos+=("$(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi

echo "checking ${#combos[@]} feature combination(s) ($MODE mode)"
fail=0
i=0
for combo in "${combos[@]}"; do
  i=$((i + 1))
  out=$(cargo check --offline --all-targets --no-default-features --features "$combo" 2>&1)
  if [[ $? -ne 0 ]]; then
    fail=$((fail + 1))
    echo "FAIL [$i/${#combos[@]}] features='$combo'"
    echo "$out" | grep -E '^(error|warning: unused)' | head -10
  fi
  if ((i % 100 == 0)); then echo "  ... $i/${#combos[@]} checked, $fail failure(s)"; fi
done

echo "-----------------------------------------------"
if ((fail == 0)); then
  echo "PASS: all ${#combos[@]} feature combinations compile"
else
  echo "FAIL: $fail / ${#combos[@]} feature combinations do not compile"
fi
exit $((fail > 0))
