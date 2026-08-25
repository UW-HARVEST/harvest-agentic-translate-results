#!/usr/bin/env bash
set -euo pipefail

crate_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$crate_root"

feature_count="$(
    awk '
        /^\[features\]$/ { in_features = 1; next }
        /^\[/ { in_features = 0 }
        in_features && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ { count++ }
        END { print count + 0 }
    ' Cargo.toml
)"
if [[ "$feature_count" != "0" ]]; then
    echo "Update feature_combinations: Cargo.toml now declares features" >&2
    exit 1
fi

# Cargo.toml currently defines no features, so its powerset is the empty set.
feature_combinations=("")

mkdir -p c_src/build
timeout 600 cmake \
    -S c_src \
    -B c_src/build \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build

for features in "${feature_combinations[@]}"; do
    timeout 600 cargo check --no-default-features --features "$features"
    timeout 600 cargo build --release --no-default-features --features "$features"
    timeout 600 cargo test --no-default-features --features "$features"
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT

nm -D --defined-only c_src/build/libtranslated_rust.so |
    awk '$2 ~ /^[TDBRWSV]$/ { print $3 }' |
    sort -u >"$c_symbols"
nm -D --defined-only target/release/libagglom_lib.so |
    awk '$2 ~ /^[TDBRWSV]$/ { print $3 }' |
    sort -u >"$rust_symbols"

if ! diff -u "$c_symbols" "$rust_symbols"; then
    echo "dynamic symbol parity failed" >&2
    exit 1
fi

echo "verified ${#feature_combinations[@]} feature combination(s)"
echo "verified $(wc -l <"$c_symbols") dynamic C/Rust symbols"
