#!/bin/bash
# Usage: ./test_config.sh <backend> <thash> <secpar>
# Example: ./test_config.sh blake simple 128f
set -e

BACKEND=$1
THASH=$2
SECPAR=$3
FEATURES="${BACKEND},${THASH},${SECPAR}"

echo "=== Testing configuration: $FEATURES ==="

cd /tmp/harvest-work-Qqa19M/translated_rust

# Map backend name for CMake (shake -> shake256 in some cases? Let's check)
CMAKE_BACKEND=$BACKEND

# Build C .so for this config
cd c_src
rm -rf build
mkdir -p build
cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DHASH_BACKEND=$CMAKE_BACKEND -DSECPAR=$SECPAR -DTHASH=$THASH 2>&1 | tail -3
cmake --build . 2>&1 | tail -3
cd /tmp/harvest-work-Qqa19M/translated_rust

# Build Rust .so for this config
timeout 120 cargo build --release --no-default-features --features "$FEATURES" 2>&1 | tail -3

# Compare symbols
echo "--- Symbol comparison ---"
(nm -D c_src/build/app/libsphincs_core_det.so 2>/dev/null | grep " T " | awk '{print $3}'; \
 nm -D c_src/build/lib/${BACKEND}/lib${BACKEND}.so 2>/dev/null | grep " T " | awk '{print $3}') | \
 grep -v "^_" | sort -u > /tmp/c_syms_${FEATURES}.txt

nm -D target/release/libsphincsplus.so 2>/dev/null | grep " T " | awk '{print $3}' | \
 grep -v "^_" | sort -u > /tmp/rust_syms_${FEATURES}.txt

MISSING=$(comm -23 /tmp/c_syms_${FEATURES}.txt /tmp/rust_syms_${FEATURES}.txt)
if [ -n "$MISSING" ]; then
    echo "MISSING SYMBOLS:"
    echo "$MISSING"
else
    echo "All symbols match (C: $(wc -l < /tmp/c_syms_${FEATURES}.txt), Rust: $(wc -l < /tmp/rust_syms_${FEATURES}.txt))"
fi

# Run tests
echo "--- Running tests ---"
timeout 600 cargo test --release --no-default-features --features "$FEATURES" --test ffi_compare -- --nocapture 2>&1 | tail -20

echo "=== Done: $FEATURES ==="
