#!/bin/bash
# Build and test a combination both in C and Rust, compare outputs
set -e
BACKEND="$1"
THASH="$2"
SECPAR="$3"
ROOT="/tmp/harvest-translate-TzX4FX/translated_rust"
cd "$ROOT/c_src"
rm -rf build
mkdir build
cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DHASH_BACKEND="$BACKEND" -DTHASH="$THASH" -DSECPAR="$SECPAR" > /dev/null 2>&1
cmake --build . > /dev/null 2>&1
C_OUT=$(./app/driver)
cd "$ROOT"
cargo build --release --quiet --no-default-features --features "$BACKEND,$THASH,$SECPAR" --bin driver 2>&1 > /dev/null
R_OUT=$(./target/release/driver)
if [[ "$C_OUT" == "$R_OUT" ]]; then
  echo "OK $BACKEND/$THASH/$SECPAR: $C_OUT"
else
  echo "MISMATCH $BACKEND/$THASH/$SECPAR"
  echo "  C:    $C_OUT"
  echo "  Rust: $R_OUT"
fi
