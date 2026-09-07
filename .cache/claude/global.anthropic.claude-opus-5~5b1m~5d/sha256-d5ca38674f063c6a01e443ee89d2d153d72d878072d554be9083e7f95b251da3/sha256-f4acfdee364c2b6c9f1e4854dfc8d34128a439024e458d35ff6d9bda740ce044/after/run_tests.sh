#!/bin/bash
# Differential test driver over EVERY valid feature combination.
#
#   ./run_tests.sh                    -> all 60 cargo feature combinations
#   ./run_tests.sh blake 128f simple  -> just that one
#
# Combinations are partitioned over $JOBS workers; each worker owns one
# CARGO_TARGET_DIR slot and walks its combinations sequentially, so no two
# concurrent builds ever share an output directory.
R="$(cd "$(dirname "$0")" && pwd)"

BACKENDS=${BACKENDS:-"haraka sha2 shake shake256 blake"}
THASHES=${THASHES:-"robust simple"}
SECPARS=${SECPARS:-"128s 128f 192s 192f 256s 256f"}
JOBS=${JOBS:-10}

if [ $# -eq 3 ]; then
  exec "$R/run_one.sh" "$1" "$2" "$3" 0
fi

combos=()
for b in $BACKENDS; do for t in $THASHES; do for s in $SECPARS; do
  combos+=("$b $s $t")
done; done; done

n=${#combos[@]}
echo "$n combinations, $JOBS workers"
res="$R/.results"
rm -rf "$res"; mkdir -p "$res"

worker() {
  local slot=$1
  local k=$slot
  while [ "$k" -lt "$n" ]; do
    set -- ${combos[$k]}
    "$R/run_one.sh" "$1" "$2" "$3" "$slot" > "$res/$k.txt" 2>&1
    k=$((k + JOBS))
  done
}

for slot in $(seq 0 $((JOBS-1))); do
  worker "$slot" &
done
wait

pass=0; fail=0
for k in $(seq 0 $((n-1))); do
  if grep -q '^ok' "$res/$k.txt" 2>/dev/null; then pass=$((pass+1)); else fail=$((fail+1)); fi
  cat "$res/$k.txt" 2>/dev/null
done
echo "-----------------------------------------"
echo "combinations passed: $pass   failed: $fail"
[ "$fail" -eq 0 ]
