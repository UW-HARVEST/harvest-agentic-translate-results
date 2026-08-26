#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$root"

timeout 600 cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build

# Cargo.toml declares no features, so the power set contains only the empty set.
feature_combinations=("")
for features in "${feature_combinations[@]}"; do
    args=(--no-default-features)
    if [[ -n "$features" ]]; then
        args+=(--features "$features")
    fi
    timeout 600 cargo check "${args[@]}"
    timeout 600 cargo build --release "${args[@]}"
    timeout 600 cargo test "${args[@]}"
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT
nm -D --defined-only c_src/build/libtranslated_rust.so |
    awk '{print $3}' | sort -u >"$c_symbols"
nm -D --defined-only target/release/libconvert_pix_lib.so |
    awk '{print $3}' | sort -u >"$rust_symbols"

if ! diff -u "$c_symbols" "$rust_symbols"; then
    echo "dynamic symbol surfaces differ" >&2
    exit 1
fi
