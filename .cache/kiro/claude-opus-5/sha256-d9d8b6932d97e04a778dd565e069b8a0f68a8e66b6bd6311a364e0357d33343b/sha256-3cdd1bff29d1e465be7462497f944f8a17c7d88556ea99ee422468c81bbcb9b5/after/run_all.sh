#!/bin/bash
# Phases B-D across every feature combination.
#
# 3 OP values x 8 REPEAT values = 24 combinations, matching the CMake cache
# variables 1:1 (-DOP=add|sub|mul, -DREPEAT=0..7). Each combination re-runs the
# whole differential suite; the harness rebuilds the matching C .so and Rust
# cdylib on demand, so every run compares like against like.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation" || exit 1

OPS="${OPS:-add sub mul}"
REPS="${REPS:-0 1 2 3 4 5 6 7}"
LOGDIR="$ROOT/cbuild/testlogs"
mkdir -p "$LOGDIR"

pass=0
fail=0
failed_combos=""
for op in $OPS; do
  for r in $REPS; do
    log="$LOGDIR/test_${op}_${r}.log"
    if timeout 600 cargo test --release --no-default-features --features "$op,$r" \
         -- --test-threads=1 > "$log" 2>&1; then
      n=$(grep -c '^test .* ok$' "$log")
      printf '%-10s PASS (%s tests)\n' "${op},${r}" "$n"
      pass=$((pass + 1))
    else
      printf '%-10s FAIL  (see %s)\n' "${op},${r}" "$log"
      grep -E '^test .* FAILED|^assertion|left:|right:' "$log" | head -8
      fail=$((fail + 1))
      failed_combos="$failed_combos ${op},${r}"
    fi
  done
done

echo
echo "combinations passed: $pass   failed: $fail"
[ -n "$failed_combos" ] && echo "failed:$failed_combos"
[ "$fail" -eq 0 ]
