#!/bin/bash
# Build & test every (OP, REPEAT) combination.
set -u
fail=0
for op in add sub mul; do
    for n in 0 1 2 3 4 5 6 7; do
        echo "=========================================="
        echo "Testing OP=$op REPEAT=$n"
        echo "=========================================="
        # cargo test only links the rlib for the test binary, so we have to
        # explicitly build the cdylib (libdriver.so) for the test harness to
        # libloading::Library::new() it.
        if ! timeout 600 cargo build --release --no-default-features --features "${op},${n}" 2>&1 | tail -3; then
            echo "*** BUILD FAILED ${op},${n} ***"
            fail=1
            continue
        fi
        if ! timeout 600 cargo test --release --no-default-features --features "${op},${n}" -- --test-threads=1 2>&1 | tail -20; then
            echo "*** TEST FAILED ${op},${n} ***"
            fail=1
        fi
    done
done
if [[ $fail -ne 0 ]]; then
    echo "Some combinations FAILED"
    exit 1
fi
echo "All combinations passed."
