#!/bin/sh
# One-shot differential verification of the Rust translation against the C.
#
#   ./run_all.sh          # default configuration only
#   ./check_features.sh   # every build configuration (this script + more)
set -e
cd "$(dirname "$0")"

echo "== building the C shared library =="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )

echo "== building the Rust cdylib (release) =="
cargo build --release --offline >/dev/null

echo "== Phase A/D: symbol parity =="
./check_symbols.sh

echo "== Phases B + C + D: differential tests =="
cargo test --offline -- --test-threads=4

echo
echo "OK"
