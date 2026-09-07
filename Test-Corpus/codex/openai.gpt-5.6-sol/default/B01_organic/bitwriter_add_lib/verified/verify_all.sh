#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

c_so="../c_src/build/libharvest-work-UTQhvA.so"
rust_so="$PWD/target/release/libbitwriter_add_lib.so"

for configuration in default no-default-features; do
    cargo_args=()
    if [[ "$configuration" == "no-default-features" ]]; then
        cargo_args+=(--no-default-features)
    fi

    echo "== $configuration: cargo check =="
    timeout 600 cargo check "${cargo_args[@]}"

    echo "== $configuration: release cdylib =="
    timeout 600 cargo build --release "${cargo_args[@]}"

    echo "== $configuration: dynamic symbol parity =="
    missing_symbols="$(
        comm -23 \
            <(nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u) \
            <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u)
    )"
    if [[ -n "$missing_symbols" ]]; then
        echo "Rust cdylib is missing C symbols:"
        echo "$missing_symbols"
        exit 1
    fi

    echo "== $configuration: tests =="
    timeout 600 env BITWRITER_RUST_SO="$rust_so" \
        cargo test "${cargo_args[@]}" -- --test-threads=1
done
