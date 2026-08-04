#!/bin/bash
# Build C source as a shared library for every (OP, REPEAT) combination.
set -e
mkdir -p c_libs
for op in add sub mul; do
    for n in 0 1 2 3 4 5 6 7; do
        out="c_libs/lib_${op}_${n}.so"
        gcc -O2 -shared -fPIC -DOP=${op} -DREPEAT=${n} c_src/src/mdcore.c -o "$out"
    done
done
echo "Built C libraries:"
ls -1 c_libs/
