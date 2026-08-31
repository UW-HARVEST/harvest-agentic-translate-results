#!/usr/bin/env bash
# Build BOTH shared libraries, then run the differential test suite.
# The Rust crate is only ever exercised through its cdylib exports, so the
# release .so must exist before the tests run.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build "$ROOT/c_src/build" -j "$(nproc)" >/dev/null
echo "   -> $ROOT/c_src/build/libzstd.so"

echo "== building Rust cdylib (release) =="
cargo build --release --offline --manifest-path "$ROOT/translation/Cargo.toml"
echo "   -> $ROOT/translation/target/release/libzstd.so"

mkdir -p "$ROOT/tmp"
echo "== symbol parity =="
nm -D --defined-only "$ROOT/c_src/build/libzstd.so"          | awk '{print $3}' | sort -u > "$ROOT/tmp/.c_syms.$$"
nm -D --defined-only "$ROOT/translation/target/release/libzstd.so" | awk '{print $3}' | sort -u > "$ROOT/tmp/.r_syms.$$"
missing=$(comm -23 "$ROOT/tmp/.c_syms.$$" "$ROOT/tmp/.r_syms.$$" | wc -l)
echo "   C=$(wc -l < "$ROOT/tmp/.c_syms.$$") Rust=$(wc -l < "$ROOT/tmp/.r_syms.$$") missing=$missing"
rm -f "$ROOT/tmp/.c_syms.$$" "$ROOT/tmp/.r_syms.$$"
[ "$missing" -eq 0 ] || { echo "SYMBOL PARITY FAILED"; exit 1; }

echo "== differential tests =="
cargo test --release --offline --manifest-path "$ROOT/translation/Cargo.toml" "$@"
