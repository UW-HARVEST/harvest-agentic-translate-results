#!/bin/sh
# Rebuild both shared objects, then run the differential suite.
#
# --test-threads=1 is MANDATORY: the C `dtoa()` keeps a process-global,
# non-thread-safe freelist, and `json_set_alloc_funcs*` mutates process-global
# allocator hooks, so concurrent tests would race inside the C library itself.
set -e
ROOT=$(cd "$(dirname "$0")/.." && pwd)

mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null )
( cd "$ROOT/translation" && cargo build --offline --release )

cd "$ROOT/translation"
exec cargo test --offline --release "$@" -- --test-threads=1
