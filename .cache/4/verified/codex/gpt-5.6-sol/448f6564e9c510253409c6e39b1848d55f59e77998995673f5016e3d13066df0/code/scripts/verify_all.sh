#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$root"

mkdir -p c_src/build
(
    cd c_src/build
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    timeout 600 cmake --build .
)

# Cargo.toml declares no features, so the feature power set contains only "".
printf '%s\n' '' |
while IFS= read -r features; do
    timeout 600 cargo check --no-default-features --features "$features"
    timeout 600 cargo build --no-default-features --features "$features"
    timeout 600 cargo test --no-default-features --features "$features"
    timeout 600 cargo build --release --no-default-features --features "$features"
done

nm -D --defined-only c_src/build/libtranslated_rust.so |
    awk '$2 ~ /^[A-Za-z]$/ {print $3}' |
    LC_ALL=C sort -u > c_exports.txt
nm -D --defined-only target/release/libupdate_frame_header_lib.so |
    awk '$2 ~ /^[A-Za-z]$/ {print $3}' |
    LC_ALL=C sort -u > rust_exports.txt
diff -u c_exports.txt rust_exports.txt > export_diff.txt

test "$(grep -c '^| C[0-9].*| \[x\] |$' CONFIGS.md)" -eq 7140
test "$(grep -c '^| C[0-9].*| \[ \] |$' CONFIGS.md || true)" -eq 0
test "$(grep -c '^| B1 .*| \[x\] |$' CONFIGS.md)" -eq 1
test "$(grep -c '^| `update_frame_header`.*\[x\]' SYMBOLS.md)" -eq 1
test "$(grep -c '^| G[124].*\[x\]' ERRORS.md)" -eq 3
test ! -s export_diff.txt

printf '%s\n' 'All feature combinations, differential tests, and symbol checks passed.'
