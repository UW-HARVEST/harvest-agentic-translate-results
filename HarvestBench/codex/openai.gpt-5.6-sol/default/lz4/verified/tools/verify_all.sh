#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root_dir="$(cd "$crate_dir/.." && pwd)"
c_build_dir="$root_dir/c_src/build"

cd "$crate_dir"

timeout 600 cmake -S "$root_dir/c_src" -B "$c_build_dir" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build "$c_build_dir"

timeout 600 cargo check
timeout 600 cargo build --release
timeout 600 cargo test --release -- --test-threads=1

timeout 600 cargo check --no-default-features
timeout 600 cargo build --release --no-default-features
timeout 600 cargo test --release --no-default-features -- --test-threads=1

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
mentioned_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols" "$mentioned_symbols"' EXIT

nm -D --defined-only "$c_build_dir/liblz4.so" |
    awk '{print $3}' | sort -u > "$c_symbols"
nm -D --defined-only "$crate_dir/target/release/liblz4.so" |
    awk '{print $3}' | sort -u > "$rust_symbols"

test "$(wc -l < "$c_symbols")" -eq 143
test "$(wc -l < "$rust_symbols")" -eq 143
test -z "$(comm -3 "$c_symbols" "$rust_symbols")"

test "$(grep -c '^| [0-9]' SYMBOLS.md)" -eq 143
test "$(grep -c '^| [0-9]' CONFIGS.md)" -eq 143
test "$(grep -c '^| [0-9]' ERRORS.md)" -eq 121
test "$(grep -c '^| [0-9].*\[x\] |$' CONFIGS.md)" -eq 143
test "$(grep -c '^| [0-9].*\[x\] |$' ERRORS.md)" -eq 121
test "$(grep -c 'Missing C symbols in Rust: \*\*0\*\*' SYMBOLS.md)" -eq 1

rg -o 'LZ4[A-Za-z0-9_]+' tests/differential.rs | sort -u > "$mentioned_symbols"
test -z "$(comm -23 "$c_symbols" "$mentioned_symbols")"

if ldd -r target/release/liblz4.so 2>&1 | grep -q 'undefined symbol'; then
    echo "Rust shared library has unresolved dynamic symbols" >&2
    exit 1
fi

echo "verification complete: 143 symbols, 143 configurations, 121 error rows"
