#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cd "$root"

timeout 600 cmake -S c_src -B c_src/build \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build

# Cargo.toml has no [features] entries, so the power set contains only {}.
feature_combinations=("")
for features in "${feature_combinations[@]}"; do
    timeout 600 cargo check --no-default-features --features "$features"
    timeout 600 cargo build --release --no-default-features --features "$features"
    timeout 600 cargo test --no-default-features --features "$features"
done

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT

nm -D --defined-only c_src/build/libdriver.so |
    awk '$2 ~ /^[A-Za-z]$/ { print $3 }' |
    sort -u >"$temporary_directory/c-symbols"
nm -D --defined-only target/release/libdriver.so |
    awk '$2 ~ /^[A-Za-z]$/ { print $3 }' |
    sort -u >"$temporary_directory/rust-symbols"

if ! diff -u \
    "$temporary_directory/c-symbols" \
    "$temporary_directory/rust-symbols"; then
    echo "C and Rust dynamic symbol surfaces differ" >&2
    exit 1
fi
