#!/bin/bash
# Exhaustive kernel sweep: verify perform_expensive_operations (i.e. f^100)
# agrees between the C and Rust .so for EVERY one of the 2^32 int values.
#
# The array holds 262144 ints, so 16384 contiguous windows of
# `array[i] = base + i` cover the whole domain exactly once.
#
# usage: sweep.sh <shard-index> <shard-count> <outfile>
set -u
here="$(cd "$(dirname "$0")" && pwd)"
C="$here/../c_src/build/liblong.so"
R="$here/../translation/target/release/liblong.so"
shard=$1
nshards=$2
out=$3

: > "$out"
win=262144
total=16384
for ((w = shard; w < total; w += nshards)); do
    base=$(( -2147483648 + w * win ))
    ops=("fill:seq:$base" "pxo:1" "hash")
    ch=$("$here/runner" "$C" "${ops[@]}") || { echo "C FAIL base=$base" >> "$out"; exit 1; }
    rh=$("$here/runner" "$R" "${ops[@]}") || { echo "R FAIL base=$base" >> "$out"; exit 1; }
    if [ "$ch" != "$rh" ]; then
        echo "MISMATCH base=$base C=$ch R=$rh" >> "$out"
    fi
done
echo "SHARD $shard DONE" >> "$out"
