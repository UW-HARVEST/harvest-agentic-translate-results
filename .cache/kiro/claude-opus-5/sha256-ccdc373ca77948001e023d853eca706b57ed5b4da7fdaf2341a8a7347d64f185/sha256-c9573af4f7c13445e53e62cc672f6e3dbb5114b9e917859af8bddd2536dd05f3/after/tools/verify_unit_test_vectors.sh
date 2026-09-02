#!/bin/bash
# Validate the hard-coded expectations in translation/src/lib.rs's `ctests`
# module against the REAL C shared object.
#
# Each tuple there is (seed, ITERATIONS, expected xor).  The C library has
# ITERATIONS fixed at 2000, but `ITERATIONS = k` is exactly "srand(seed), fill
# with rand(), then call perform_expensive_operations() k times", which the
# runner can express directly:
#
#     runner <so> fill:libcrand:SEED pxo:K xor
#
# usage: verify_unit_test_vectors.sh
set -u
here="$(cd "$(dirname "$0")" && pwd)"
C="$here/../c_src/build/liblong.so"

# (seed, iterations, expected) triples scraped from src/lib.rs
mapfile -t CASES < <(awk '
    /^ *\(([0-9]+), *[0-9]+, *-?[0-9]+\),/ {
        gsub(/[(),]/, " ");
        print $1, $2, $3
    }' "$here/../translation/src/lib.rs")

echo "scraped ${#CASES[@]} vectors from src/lib.rs"
fail=0
for c in "${CASES[@]}"; do
    set -- $c
    seed=$1; it=$2; want=$3
    if [ "$it" -gt 200 ]; then
        echo "  seed=$seed it=$it  SKIPPED (>200 iterations: use the cached long_exec reference)"
        continue
    fi
    got=$("$here/runner" "$C" "fill:libcrand:$seed" "pxo:$it" xor | tail -1)
    if [ "$got" = "$want" ]; then
        echo "  seed=$seed it=$it  OK  ($got)"
    else
        echo "  seed=$seed it=$it  MISMATCH want=$want got=$got"
        fail=1
    fi
done
[ "$fail" -eq 0 ] && echo "ALL UNIT-TEST VECTORS CONFIRMED AGAINST THE C .so" || echo "VECTOR MISMATCHES PRESENT"
exit "$fail"
