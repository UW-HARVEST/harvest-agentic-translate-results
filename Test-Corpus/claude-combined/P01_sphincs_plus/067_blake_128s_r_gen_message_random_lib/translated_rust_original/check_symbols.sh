#!/bin/bash
set -e
ROOT="/tmp/harvest-translate-TzX4FX/translated_rust"
BACKEND="$1"
THASH="$2"
SECPAR="$3"

cd "$ROOT/c_src"
rm -rf build
mkdir -p build
cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DHASH_BACKEND="$BACKEND" -DTHASH="$THASH" -DSECPAR="$SECPAR" > /dev/null 2>&1
cmake --build . > /dev/null 2>&1

cd "$ROOT"
cargo build --quiet --no-default-features --features "$BACKEND,$THASH,$SECPAR" 2>&1 > /dev/null

# Get all relevant symbols from C and Rust
nm -D --defined-only "c_src/build/lib/$BACKEND/lib$BACKEND.so" "c_src/build/app/libsphincs_core_det.so" 2>&1 | awk '{print $3}' | grep -E '^(SPX_|crypto_|randombytes|seedexpander|AES256|sha256|sha512|shake256|shake128|sha3_|blake256|blake512)' | sort | uniq > /tmp/c_syms.txt
nm -D --defined-only target/debug/libsphincs_plus.so | awk '{print $3}' | grep -E '^(SPX_|crypto_|randombytes|seedexpander|AES256|sha256|sha512|shake256|shake128|sha3_|blake256|blake512)' | sort > /tmp/rust_syms.txt

diff_in_c=$(comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt)
if [[ -z "$diff_in_c" ]]; then
  echo "OK $BACKEND/$THASH/$SECPAR: all C symbols exported by Rust"
else
  echo "MISSING $BACKEND/$THASH/$SECPAR: symbols missing from Rust .so:"
  echo "$diff_in_c"
fi
