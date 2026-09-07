#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
c_so="$(cd "$crate_dir/../c_src/build" && pwd)/libdriver.so"
rust_so="$crate_dir/target/release/libdriver.so"

cd "$crate_dir"

if [[ ! -f "$c_so" ]]; then
    echo "missing C shared object: $c_so" >&2
    exit 1
fi

run_mode() {
    local mode="$1"
    shift

    echo "verifying Cargo mode: $mode"
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    DRIVER_C_SO="$c_so" DRIVER_RUST_SO="$rust_so" \
        timeout 600 cargo test "$@" -- --test-threads=1
}

run_mode default
run_mode no-default-features --no-default-features

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT

nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u >"$c_symbols"
nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u >"$rust_symbols"
diff -u "$c_symbols" "$rust_symbols"

echo "verification complete"
