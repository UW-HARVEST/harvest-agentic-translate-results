#!/bin/sh
# Build the cdylib, stamp a content hash of src/*.rs next to it, then run the
# differential test suite.
#
# WHY THIS SCRIPT EXISTS: `cargo test` does NOT rebuild the cdylib. The
# integration tests reach `libmujs.so` only through `dlopen`, so they have no
# cargo dependency on the lib target and cargo has no reason to rebuild it.
# Running `cargo test` after editing `src/*.rs` therefore tests the PREVIOUS
# build and every test passes vacuously. Always go through this script.
set -e
cd "$(dirname "$0")"

FEATURES="${FEATURES:---no-default-features}"

echo "=== building C reference library ==="
if [ ! -f c_src/build/libmujs.so ]; then
    mkdir -p c_src/build
    (cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && \
        cmake --build . >/dev/null)
fi

echo "=== building Rust cdylib ($FEATURES) ==="
cargo build $FEATURES

SO_DIR=$(dirname "$(ls target/debug/libmujs.so)")

# Content hash, identical to `src_hash()` in tests/common/mod.rs.
./stamp_hash.sh
echo "stamped $SO_DIR/.src_hash = $(cat "$SO_DIR/.src_hash")"

echo "=== running differential tests ($FEATURES) ==="
if [ $# -gt 0 ]; then
    exec cargo test $FEATURES "$@"
else
    exec cargo test $FEATURES
fi
