#!/bin/bash
# Differential fuzz runner: compares the C library against the Rust translation
# over many random seeds, in parallel.
#
# Usage: ./run_fuzz.sh [first_seed] [last_seed] [iters_per_seed]
set -u
cd "$(dirname "$0")"

FIRST=${1:-1}
LAST=${2:-48}
ITERS=${3:-400}

C_LIB=/tmp/cb3tiS/libpcre2.so
R_LIB=../translation/target/release/libpcre2.so
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/c" "$WORK/r"
cp "$C_LIB" "$WORK/c/libpcre2.so"
cp "$R_LIB" "$WORK/r/libpcre2.so"
cp fuzz "$WORK/c/fuzz"
cp fuzz "$WORK/r/fuzz"

run_one() {
  local seed=$1
  ( cd "$WORK/c" && LD_LIBRARY_PATH=. timeout 600 ./fuzz "$seed" "$ITERS" > "$WORK/out_c_$seed" 2>&1 )
  local ec=$?
  ( cd "$WORK/r" && LD_LIBRARY_PATH=. timeout 600 ./fuzz "$seed" "$ITERS" > "$WORK/out_r_$seed" 2>&1 )
  local er=$?
  if [ "$ec" != "$er" ]; then
    echo "seed $seed EXIT MISMATCH c=$ec r=$er"
  fi
  if cmp -s "$WORK/out_c_$seed" "$WORK/out_r_$seed"; then
    echo "seed $seed OK"
  else
    echo "seed $seed DIFF ($(diff "$WORK/out_c_$seed" "$WORK/out_r_$seed" | grep -c '^[<>]') lines)"
    diff "$WORK/out_c_$seed" "$WORK/out_r_$seed" > "diff_seed_$seed.txt"
    cp "$WORK/out_c_$seed" "out_c_$seed.txt"
    cp "$WORK/out_r_$seed" "out_r_$seed.txt"
  fi
}
export -f run_one
export WORK ITERS

seq "$FIRST" "$LAST" | xargs -P 16 -I{} bash -c 'run_one {}'
