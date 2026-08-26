#!/bin/bash
# Builds both implementations and runs the whole differential suite for every
# build-time configuration (see SYMBOLS.md: the crate has no [features], so the
# complete matrix is "default" and "--no-default-features").
set -u
cd "$(dirname "$0")"

CARGO_FLAGS_LIST=("" "--no-default-features")
fail=0

echo "=== building the C sources (unmodified c_src) ==="
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . ) \
    || { echo "C driver build FAILED"; exit 1; }
mkdir -p build_c
gcc -shared -fPIC -O2 -I c_src/include -o build_c/libdag_c.so c_src/src/lib.c \
    || { echo "C shared library build FAILED"; exit 1; }

for flags in "${CARGO_FLAGS_LIST[@]}"; do
    label="${flags:-default}"
    echo
    echo "=== cargo check [$label] ==="
    timeout 600 cargo check --offline --all-targets $flags 2>&1 | tail -3 || fail=1

    echo "=== cargo build [$label] ==="
    timeout 600 cargo build --offline $flags 2>&1 | tail -2 || fail=1

    echo "=== symbol parity [$label] ==="
    diff <(nm -D --defined-only build_c/libdag_c.so       | awk '{print $3}' | sort) \
         <(nm -D --defined-only target/debug/libdag_rs.so | awk '{print $3}' | sort) \
        && echo "symbols: identical ($(nm -D --defined-only build_c/libdag_c.so | wc -l) exported)" \
        || { echo "SYMBOL DIFF"; fail=1; }

    echo "=== cargo test [$label] ==="
    log="logs/test-${label// /_}.log"
    mkdir -p logs
    timeout 600 cargo test --offline $flags > "$log" 2>&1
    if [ $? -ne 0 ]; then fail=1; fi
    grep -E "^(running|test result|error|warning: unused)" "$log"
    grep -E "FAILED|panicked" "$log" | head -20
    echo "(full output: $log)"
done

echo
echo "=== release build (panic = abort) ==="
timeout 600 cargo build --release --offline 2>&1 | tail -2 || fail=1

echo
if [ "$fail" = 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "FAILURES PRESENT"
    exit 1
fi
