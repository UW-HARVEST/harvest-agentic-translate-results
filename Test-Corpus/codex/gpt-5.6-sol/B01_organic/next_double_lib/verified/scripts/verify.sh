#!/usr/bin/env bash
set -euo pipefail

crate_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$crate_root"

timeout 600 cmake \
    -S c_src \
    -B c_src/build \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build

# Cargo.toml has no [features] table, so the empty set is the full matrix.
feature_combinations=("")
for features in "${feature_combinations[@]}"; do
    cargo_args=(--no-default-features)
    if [[ -n "$features" ]]; then
        cargo_args+=(--features "$features")
    fi

    timeout 600 cargo check "${cargo_args[@]}"
    # cargo test does not refresh a cdylib, which the integration test loads.
    timeout 600 cargo build "${cargo_args[@]}"
    timeout 600 cargo test "${cargo_args[@]}"
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
missing_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols" "$missing_symbols"' EXIT

nm -D --defined-only c_src/build/libtranslated_rust.so |
    awk 'NF >= 3 { print $3 }' |
    sort -u >"$c_symbols"
nm -D --defined-only target/debug/libnext_double_lib.so |
    awk 'NF >= 3 { print $3 }' |
    sort -u >"$rust_symbols"
comm -23 "$c_symbols" "$rust_symbols" >"$missing_symbols"

if [[ -s "$missing_symbols" ]]; then
    echo "Rust shared library is missing C symbols:" >&2
    cat "$missing_symbols" >&2
    exit 1
fi

echo "Verification passed: all feature combinations and C dynamic symbols."
