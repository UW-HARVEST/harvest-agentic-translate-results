#!/bin/sh
set -eu

crate_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
root_dir=$(CDPATH= cd -- "$crate_dir/.." && pwd)

mkdir -p "$root_dir/c_src/build"
(
    cd "$root_dir/c_src/build"
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    timeout 600 cmake --build .
)

cd "$crate_dir"
timeout 600 cargo check
timeout 600 cargo build --release
timeout 600 cargo test -- --test-threads=1

timeout 600 cargo check --no-default-features
timeout 600 cargo build --release --no-default-features
timeout 600 cargo test --no-default-features -- --test-threads=1

c_symbols=$(mktemp)
rust_symbols=$(mktemp)
missing=$(mktemp)
extra=$(mktemp)
trap 'rm -f "$c_symbols" "$rust_symbols" "$missing" "$extra"' EXIT HUP INT TERM

nm -D --defined-only "$root_dir/c_src/build/liblz4.so" |
    awk '{print $3}' |
    sort -u > "$c_symbols"
nm -D --defined-only "$crate_dir/target/release/liblz4.so" |
    awk '{print $3}' |
    sort -u > "$rust_symbols"
comm -23 "$c_symbols" "$rust_symbols" > "$missing"
comm -13 "$c_symbols" "$rust_symbols" > "$extra"

test ! -s "$missing"
test ! -s "$extra"
test "$(wc -l < "$c_symbols")" -eq 143
test "$(grep -c '^| [0-9]' SYMBOLS.md)" -eq 143
test "$(grep -c '^| [0-9]' CONFIGS.md)" -eq 143
test "$(grep -c '^| [0-9]' ERRORS.md)" -ge 290
test "$(grep -c '^| [0-9].*| \\[ \\] |$' CONFIGS.md || true)" -eq 0
test "$(grep -c '^| [0-9].*| \\[ \\] |$' ERRORS.md || true)" -eq 0
