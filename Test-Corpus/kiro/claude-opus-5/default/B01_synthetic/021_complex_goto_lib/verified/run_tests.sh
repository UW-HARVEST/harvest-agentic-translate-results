#!/usr/bin/env bash
# Full verification run: build both libraries, check symbol parity, then run the
# differential suite against BOTH the debug and the release Rust .so (release
# uses `panic = "abort"`, so it is a genuinely different artifact).
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)

echo "=== 1. build the C shared library ==="
(cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
  && cmake --build . 2>&1 | tail -3)

echo
echo "=== 2. build the Rust cdylib (debug + release) ==="
cargo build --quiet 2>&1 | tail -3
cargo build --quiet --release 2>&1 | tail -3

echo
echo "=== 3. symbol parity ==="
./check_symbols.sh || exit 1

rc=0
for profile in debug release; do
  echo
  echo "=== 4. differential suite against target/$profile/libdriver.so ==="
  if ! DRIVER_RUST_SO="$PWD/target/$profile/libdriver.so" \
       timeout 600 cargo test -- --test-threads=1 2>&1 | tail -12; then
    rc=1
  fi
done

echo
[ "$rc" -eq 0 ] && echo "VERIFICATION PASSED" || echo "VERIFICATION FAILED"
exit "$rc"
