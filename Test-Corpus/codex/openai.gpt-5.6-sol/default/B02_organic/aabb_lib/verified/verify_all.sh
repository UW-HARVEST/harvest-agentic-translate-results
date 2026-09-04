#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

run_mode() {
    local label=$1
    shift
    echo "== checking ${label} =="
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test "$@"
}

run_mode default
run_mode no-default-features --no-default-features

c_symbols=$(mktemp)
rust_symbols=$(mktemp)
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT

nm -D --defined-only ../c_src/build/libharvest-work-3Nv5PV.so |
    awk '{print $3}' | sort -u >"$c_symbols"
nm -D --defined-only target/release/libaabb_lib.so |
    awk '{print $3}' | sort -u >"$rust_symbols"

missing=$(comm -23 "$c_symbols" "$rust_symbols")
extra=$(comm -13 "$c_symbols" "$rust_symbols")
test -z "$missing"
test -z "$extra"
ldd -r target/release/libaabb_lib.so
