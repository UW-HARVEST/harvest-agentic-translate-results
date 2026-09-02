#!/bin/bash
# Build the allocator interposer and run the differential suite with it
# preloaded, so the malloc/free-accounting tests become active.
set -eu
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

OUT="$ROOT/target/malloc_trace.so"
mkdir -p "$ROOT/target"
cc -shared -fPIC -O2 -o "$OUT" tests/support/malloc_trace.c -ldl
echo "built $OUT"

timeout 600 cargo build --release
# --test-threads=1 because the interposer's counters are process-global.
MALLOC_TRACE_SO="$OUT" LD_PRELOAD="$OUT" \
  RUST_DRIVER_SO="$ROOT/target/release/libdriver.so" \
  timeout 600 cargo test --release -- --test-threads=1
