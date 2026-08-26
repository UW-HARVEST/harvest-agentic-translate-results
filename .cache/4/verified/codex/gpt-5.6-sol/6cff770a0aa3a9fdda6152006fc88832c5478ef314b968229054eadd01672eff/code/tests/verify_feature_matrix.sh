#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

timeout 600 cmake -S c_src -B c_src/build \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build

# Cargo.toml has no [features] entries, so the power set is the empty set.
feature_combinations=("")
for features in "${feature_combinations[@]}"; do
    timeout 600 cargo check --no-default-features --features "$features"
    timeout 600 cargo build --release --no-default-features --features "$features"
    timeout 600 cargo test --no-default-features --features "$features" -- \
        --test-threads=1
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
symbol_diff="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols" "$symbol_diff"' EXIT

nm -D --defined-only --format=posix c_src/build/libtranslated_rust.so |
    awk '$2 ~ /^[TDBR]$/ { print $1 }' | LC_ALL=C sort -u >"$c_symbols"
nm -D --defined-only --format=posix target/release/libnormalize_lib.so |
    awk '$2 ~ /^[TDBR]$/ { print $1 }' | LC_ALL=C sort -u >"$rust_symbols"

if ! diff -u "$c_symbols" "$rust_symbols" >"$symbol_diff"; then
    echo "C/Rust defined-symbol mismatch:" >&2
    cat "$symbol_diff" >&2
    exit 1
fi

if ldd -r target/release/libnormalize_lib.so 2>&1 |
    grep -q 'undefined symbol'; then
    echo "Rust shared object has unresolved symbols:" >&2
    ldd -r target/release/libnormalize_lib.so >&2
    exit 1
fi

echo "feature combinations: ${#feature_combinations[@]}"
echo "missing C symbols: 0"
echo "unresolved Rust symbols: 0"
