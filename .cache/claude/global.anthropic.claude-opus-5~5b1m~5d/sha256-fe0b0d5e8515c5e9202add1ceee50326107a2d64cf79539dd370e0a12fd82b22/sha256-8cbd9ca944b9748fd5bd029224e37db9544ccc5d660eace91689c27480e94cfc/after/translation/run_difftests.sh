#!/bin/sh
# Rebuild the C .so and the Rust .so (release, the artifact the tests dlopen),
# then run the differential test suite.
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(dirname "$here")

# --- C shared object -------------------------------------------------
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )

# --- Rust shared object ---------------------------------------------
( cd "$here" && cargo build --offline --release )

# --- symbol parity ---------------------------------------------------
diff <(nm -D --defined-only "$root/c_src/build/libdriver.so"      | awk '{print $3}' | sort) \
     <(nm -D --defined-only "$here/target/release/libdriver.so"   | awk '{print $3}' | sort) \
  && echo "symbol parity: OK"

# --- tests -----------------------------------------------------------
cd "$here"
exec cargo test --offline "$@"
