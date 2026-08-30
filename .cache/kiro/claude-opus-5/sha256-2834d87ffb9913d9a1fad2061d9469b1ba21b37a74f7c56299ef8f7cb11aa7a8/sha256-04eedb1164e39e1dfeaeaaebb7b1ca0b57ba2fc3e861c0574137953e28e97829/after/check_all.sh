#!/usr/bin/env bash
# Step 1/2: enumerate every valid feature combination from Cargo.toml and run
# `cargo check` on each. Usage: ./check_all.sh [cargo-subcommand ...]
set -u
cd "$(dirname "$0")/translation"

CMD=${1:-check}
shift || true

combos=()
# default (no features at all): mdmacros.h #ifndef fallbacks -> OP=add REPEAT=5
combos+=("")
# canonical OP x REPEAT
for OP in add sub mul; do for R in 0 1 2 3 4 5 6 7; do combos+=("$OP,$R"); done; done
# readable aliases
for OP in op_add op_sub op_mul; do
  for R in repeat_0 repeat_1 repeat_2 repeat_3 repeat_4 repeat_5 repeat_6 repeat_7; do
    combos+=("$OP,$R")
  done
done
# single-feature sets (the other variable falls back to its #ifndef default)
for f in add sub mul 0 1 2 3 4 5 6 7 op_add op_sub op_mul \
         repeat_0 repeat_1 repeat_2 repeat_3 repeat_4 repeat_5 repeat_6 repeat_7; do
  combos+=("$f")
done
# every feature at once (documented precedence: mul > sub > add, lowest REPEAT)
combos+=("add,sub,mul,0,1,2,3,4,5,6,7,op_add,op_sub,op_mul,repeat_0,repeat_1,repeat_2,repeat_3,repeat_4,repeat_5,repeat_6,repeat_7")

fail=0
for c in "${combos[@]}"; do
  if [[ -z "$c" ]]; then
    label="<default>"; args=(--no-default-features)
  else
    label="$c"; args=(--no-default-features --features "$c")
  fi
  if ! out=$(timeout 600 cargo "$CMD" "${args[@]}" "$@" 2>&1); then
    echo "FAIL [$CMD] features=$label"
    printf '%s\n' "$out" | grep -E '^(error|warning: unused)' | head -20 | sed 's/^/    /'
    fail=1
  fi
done
[[ $fail -eq 0 ]] && echo "ALL ${#combos[@]} FEATURE COMBINATIONS OK ($CMD)"
exit $fail
