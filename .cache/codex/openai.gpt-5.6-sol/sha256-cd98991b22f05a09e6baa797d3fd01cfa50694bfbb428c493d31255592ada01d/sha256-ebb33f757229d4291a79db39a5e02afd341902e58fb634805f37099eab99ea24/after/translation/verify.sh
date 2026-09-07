#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")" && pwd)"
cd "$crate_dir"

timeout 600 cargo check
timeout 600 cargo build --release
timeout 600 cargo test

timeout 600 cargo check --no-default-features
timeout 600 cargo build --release --no-default-features
timeout 600 cargo test --no-default-features

nm -D --defined-only ../c_src/build/libharvest-work-cp21cR.so |
    awk '$2 ~ /^[TDBR]$/ {print $3}' |
    sort -u > /tmp/fallcalc-c-symbols.txt
nm -D --defined-only target/release/libfallcalc_lib.so |
    awk '$2 ~ /^[TDBR]$/ {print $3}' |
    sort -u > /tmp/fallcalc-rust-symbols.txt

if ! diff -u /tmp/fallcalc-c-symbols.txt /tmp/fallcalc-rust-symbols.txt; then
    echo "dynamic symbol mismatch" >&2
    exit 1
fi
