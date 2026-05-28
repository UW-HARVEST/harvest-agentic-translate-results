#!/usr/bin/env bash
# Build the C library for every (HASH_BACKEND, THASH, SECPAR) combination.
set -e
cd "$(dirname "$0")/c_src"

for h in haraka sha2 shake blake; do
    for t in robust simple; do
        for s in 128s 128f 192s 192f 256s 256f; do
            build="build_${h}_${t}_${s}"
            mkdir -p "$build"
            (cd "$build" && cmake .. \
                -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
                -DHASH_BACKEND=$h \
                -DTHASH=$t \
                -DSECPAR=$s > /dev/null 2>&1 \
             && cmake --build . > /dev/null 2>&1)
            echo "[$h/$t/$s] built"
        done
    done
done
