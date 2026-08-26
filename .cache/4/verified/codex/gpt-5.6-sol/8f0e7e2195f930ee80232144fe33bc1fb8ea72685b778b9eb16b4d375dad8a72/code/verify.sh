#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

if grep -q '^\[features\]' Cargo.toml; then
    echo "Cargo feature matrix changed; update this exhaustive verifier." >&2
    exit 1
fi

cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build

# Cargo.toml has no features, so the empty set is the complete powerset.
timeout 600 cargo check --no-default-features
timeout 600 cargo build --release --no-default-features
timeout 600 cargo test --no-default-features

temporary_directory="$(mktemp -d)"
trap 'rm -rf "$temporary_directory"' EXIT

nm -D --defined-only c_src/build/libtranslated_rust.so |
    awk '$2 == "T" {print $3}' |
    sort -u >"$temporary_directory/c-symbols"
nm -D --defined-only target/release/libomni_collide_lib.so |
    awk '$2 == "T" {print $3}' |
    sort -u >"$temporary_directory/rust-symbols"
diff -u "$temporary_directory/c-symbols" "$temporary_directory/rust-symbols"

printf '%s\n' abort malloc sqrtf >"$temporary_directory/allowed-imports"
nm -D target/release/libomni_collide_lib.so |
    awk '$1 == "U" {sub(/@.*/, "", $2); print $2}' |
    sort -u >"$temporary_directory/rust-imports"
diff -u "$temporary_directory/allowed-imports" "$temporary_directory/rust-imports"
ldd -r target/release/libomni_collide_lib.so 2>&1 |
    tee "$temporary_directory/ldd"
if grep -q 'undefined symbol' "$temporary_directory/ldd"; then
    exit 1
fi

echo "Verified empty feature set: 39 exports, 0 missing, 0 extra."
