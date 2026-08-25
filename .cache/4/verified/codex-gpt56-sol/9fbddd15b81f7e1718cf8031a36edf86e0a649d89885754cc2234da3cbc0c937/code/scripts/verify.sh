#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Cargo.toml has no [features] table, so this is the complete feature matrix.
timeout 600 cargo check --no-default-features

mkdir -p c_src/build
(
    cd c_src/build
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    timeout 600 cmake --build .
)

timeout 600 cargo build --no-default-features
timeout 600 cargo test --no-default-features

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
symbol_diff="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols" "$symbol_diff"' EXIT

nm -D --defined-only c_src/build/libString_Slice.so |
    awk 'NF == 3 { print $3 }' |
    sort -u >"$c_symbols"
nm -D --defined-only target/debug/libString_Slice.so |
    awk 'NF == 3 { print $3 }' |
    sort -u >"$rust_symbols"
comm -3 "$c_symbols" "$rust_symbols" >"$symbol_diff"

if [[ -s "$symbol_diff" ]]; then
    echo "C and Rust exported symbol surfaces differ:" >&2
    cat "$symbol_diff" >&2
    exit 1
fi

ldd -r c_src/build/libString_Slice.so
ldd -r target/debug/libString_Slice.so
