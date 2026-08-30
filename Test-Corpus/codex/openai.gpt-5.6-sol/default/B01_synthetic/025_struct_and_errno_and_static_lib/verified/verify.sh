#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

c_library="../c_src/build/libdriver.so"
rust_library="$PWD/target/release/libdriver.so"

test -f "$c_library"

timeout 600 cargo check
timeout 600 cargo build --release

timeout 600 env DRIVER_RUST_SO="$rust_library" \
    cargo test --test differential -- --test-threads=1
timeout 600 env DRIVER_RUST_SO="$rust_library" \
    cargo test --no-default-features --test differential -- --test-threads=1

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

nm -D --defined-only --format=posix "$c_library" |
    awk '$2 ~ /^[A-Z]$/ { print $1 }' |
    sort -u >"$work_dir/c-symbols"
nm -D --defined-only --format=posix "$rust_library" |
    awk '$2 ~ /^[A-Z]$/ { print $1 }' |
    sort -u >"$work_dir/rust-symbols"
comm -23 "$work_dir/c-symbols" "$work_dir/rust-symbols" >"$work_dir/missing-symbols"

if [[ -s "$work_dir/missing-symbols" ]]; then
    echo "C exports missing from Rust:" >&2
    cat "$work_dir/missing-symbols" >&2
    exit 1
fi

echo "Verification complete: all differential tests pass; missing C exports: 0"
