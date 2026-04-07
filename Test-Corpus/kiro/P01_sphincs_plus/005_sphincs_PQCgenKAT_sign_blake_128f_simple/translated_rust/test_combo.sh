#!/bin/bash
set -e
BACKEND=$1
THASH=$2
SECPAR=$3

echo "=== Testing $BACKEND,$THASH,$SECPAR ==="

# Build C
cd /tmp/harvest-work-LxIBRZ/translated_rust/c_src
rm -rf build && mkdir -p build && cd build
timeout 120 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DHASH_BACKEND=$BACKEND -DSECPAR=$SECPAR -DTHASH=$THASH 2>&1 >/dev/null
timeout 120 cmake --build . 2>&1 >/dev/null

# Build Rust .so
cd /tmp/harvest-work-LxIBRZ/translated_rust
timeout 120 cargo build --release --no-default-features --features "$BACKEND,$THASH,$SECPAR" 2>&1 >/dev/null

# Compare symbols
C_SYMS=$(mktemp)
R_SYMS=$(mktemp)
(nm -D c_src/build/app/libsphincs_core_det.so | grep " T " | awk '{print $3}'; \
 nm -D c_src/build/lib/$BACKEND/lib${BACKEND}.so | grep " T " | awk '{print $3}') | sort -u | grep -v "^_" > $C_SYMS
nm -D target/release/libsphincsplus.so | grep " T " | awk '{print $3}' | sort -u | grep -v "^_" > $R_SYMS

MISSING=$(comm -23 $C_SYMS $R_SYMS)
if [ -n "$MISSING" ]; then
  echo "MISSING SYMBOLS: $MISSING"
  rm -f $C_SYMS $R_SYMS
  exit 1
fi
rm -f $C_SYMS $R_SYMS

# Run tests
LD_LIBRARY_PATH="/tmp/harvest-work-LxIBRZ/translated_rust/c_src/build/app:/tmp/harvest-work-LxIBRZ/translated_rust/c_src/build/lib/$BACKEND" \
timeout 600 cargo test --release --no-default-features --features "$BACKEND,$THASH,$SECPAR" -- --test-threads=1 2>&1 | grep -E "^(test |test result)"

echo "=== DONE $BACKEND,$THASH,$SECPAR ==="
