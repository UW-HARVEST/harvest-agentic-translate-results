#!/usr/bin/env bash
# End-to-end verification: build the C .so, build the Rust .so, check symbol
# parity, and run every differential test across every feature combination and
# both profiles.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root="$(dirname "$here")"

echo "### 1/3 build C shared library ###"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . ) || exit 1
ls -l "$root/c_src/build/libdriver.so"

echo
echo "### 2/3 build Rust cdylib ###"
( cd "$here" && timeout 600 cargo build && timeout 600 cargo build --release ) || exit 1

echo
echo "### 3/3 symbol parity + all feature combinations + both profiles ###"
"$here/scripts/check_features.sh"
