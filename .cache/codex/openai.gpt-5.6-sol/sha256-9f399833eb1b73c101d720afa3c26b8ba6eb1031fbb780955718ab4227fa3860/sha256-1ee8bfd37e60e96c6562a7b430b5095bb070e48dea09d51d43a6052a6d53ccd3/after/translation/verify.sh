#!/bin/sh
set -eu

run_configuration()
{
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test "$@"
}

run_configuration
run_configuration --no-default-features

c_symbols=$(mktemp)
rust_symbols=$(mktemp)
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT

nm -D --defined-only --extern-only ../c_src/build/libdriver.so |
    awk '{print $3}' | sort -u > "$c_symbols"
nm -D --defined-only --extern-only target/release/libdriver.so |
    awk '{print $3}' | sort -u > "$rust_symbols"

if ! diff -u "$c_symbols" "$rust_symbols"; then
    echo "dynamic symbol surfaces differ" >&2
    exit 1
fi

ldd -r target/release/libdriver.so
