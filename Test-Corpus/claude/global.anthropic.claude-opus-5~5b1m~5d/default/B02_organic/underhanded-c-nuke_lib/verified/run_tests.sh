#!/bin/sh
# Differential-test driver.
#
# Rebuilds BOTH shared objects, then runs the Phase B/C/D test suites.
# `cargo test` alone is not enough: it builds test harnesses but does not
# re-emit the `cdylib` artifact, so the Rust `.so` under test would be stale.
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)

# --- C reference .so, built exactly as the task prescribes -------------------
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )

# --- Rust .so ---------------------------------------------------------------
cd "$here"
cargo build --release --offline "$@" >/dev/null

echo "C    .so: $(ls -1 "$root"/c_src/build/lib*.so)"
echo "Rust .so: $here/target/release/libunderhanded_c_nuke_lib.so"
echo

cargo test --release --offline "$@" -- --test-threads=8
