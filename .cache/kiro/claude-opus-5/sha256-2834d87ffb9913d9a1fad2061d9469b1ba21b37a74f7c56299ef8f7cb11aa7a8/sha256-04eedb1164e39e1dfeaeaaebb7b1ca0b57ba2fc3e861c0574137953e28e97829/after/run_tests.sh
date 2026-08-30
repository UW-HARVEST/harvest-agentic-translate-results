#!/usr/bin/env bash
# Build + run the differential test suite for a group of feature combinations.
#
#   ./run_tests.sh canonical   # OP x REPEAT, the 24 configurations CMake can produce
#   ./run_tests.sh aliases     # the readable op_*/repeat_* aliases
#   ./run_tests.sh edge        # default (no features), single features, all features
#
# Each combination is built and then tested against the matching C reference in
# /tmp/cref (see build_cref.sh).
set -u
cd "$(dirname "$0")/translation"

group=${1:-canonical}
combos=()
case "$group" in
  canonical)
    for OP in add sub mul; do for R in 0 1 2 3 4 5 6 7; do combos+=("$OP,$R"); done; done ;;
  aliases)
    for OP in op_add op_sub op_mul; do
      for R in repeat_0 repeat_1 repeat_2 repeat_3 repeat_4 repeat_5 repeat_6 repeat_7; do
        combos+=("$OP,$R")
      done
    done ;;
  edge)
    combos+=("")
    for f in add sub mul 0 1 2 3 4 5 6 7 \
             op_add op_sub op_mul \
             repeat_0 repeat_1 repeat_2 repeat_3 repeat_4 repeat_5 repeat_6 repeat_7; do
      combos+=("$f")
    done
    combos+=("add,sub,mul,0,1,2,3,4,5,6,7,op_add,op_sub,op_mul,repeat_0,repeat_1,repeat_2,repeat_3,repeat_4,repeat_5,repeat_6,repeat_7") ;;
  *) echo "unknown group: $group" >&2; exit 64 ;;
esac

fail=0
for c in "${combos[@]}"; do
  if [[ -z "$c" ]]; then label="<default>"; args=(--no-default-features)
  else label="$c"; args=(--no-default-features --features "$c"); fi

  if ! out=$(timeout 300 cargo build "${args[@]}" 2>&1); then
    echo "BUILD FAIL $label"; printf '%s\n' "$out" | grep -E '^error' | head -10 | sed 's/^/    /'
    fail=1; continue
  fi
  if ! out=$(timeout 300 cargo test "${args[@]}" 2>&1); then
    echo "TEST FAIL $label"
    printf '%s\n' "$out" | grep -E 'FAILED|panicked at|^ *[a-z_]+ (matches|stays|resolves)|assertion' \
      | head -20 | sed 's/^/    /'
    fail=1
  else
    printf 'ok   %s\n' "$label"
  fi
done

if [[ $fail -eq 0 ]]; then
  echo "ALL ${#combos[@]} CONFIGURATIONS IN GROUP '$group' MATCH THE C REFERENCE"
fi
exit $fail
