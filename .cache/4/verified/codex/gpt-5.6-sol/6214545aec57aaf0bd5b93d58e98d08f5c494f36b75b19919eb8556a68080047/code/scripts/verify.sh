#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

mkdir -p c_src/build
cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build
timeout 600 cc -fPIC -shared -o c_src/build/libdriver_c.so c_src/src/lib.c

# Cargo.toml has no [features] table, so the powerset contains only empty.
feature_combinations=("")

for combination in "${feature_combinations[@]}"; do
    cargo_args=(--no-default-features)
    rustc_cfg=()
    if [[ -n "$combination" ]]; then
        cargo_args+=(--features "$combination")
        IFS=',' read -ra enabled_features <<< "$combination"
        for feature in "${enabled_features[@]}"; do
            rustc_cfg+=(--cfg "feature=\"$feature\"")
        done
    fi

    timeout 600 cargo check "${cargo_args[@]}"
    mkdir -p target/debug
    timeout 600 rustc \
        --edition=2021 \
        --crate-name driver \
        --crate-type cdylib \
        -C overflow-checks=off \
        "${rustc_cfg[@]}" \
        src/lib.rs \
        -o target/debug/libdriver.so
    timeout 600 cargo test "${cargo_args[@]}"
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
missing_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols" "$missing_symbols"' EXIT

nm -D --defined-only c_src/build/libdriver_c.so |
    awk '$2 ~ /^[TDBR]$/ { print $3 }' |
    sort -u > "$c_symbols"
nm -D --defined-only target/debug/libdriver.so |
    awk '$2 ~ /^[TDBR]$/ { print $3 }' |
    sort -u > "$rust_symbols"
comm -23 "$c_symbols" "$rust_symbols" > "$missing_symbols"

if [[ -s "$missing_symbols" ]]; then
    echo "Rust shared library is missing C symbols:" >&2
    cat "$missing_symbols" >&2
    exit 1
fi

echo "Verified empty feature set; C-to-Rust symbol diff is empty."

