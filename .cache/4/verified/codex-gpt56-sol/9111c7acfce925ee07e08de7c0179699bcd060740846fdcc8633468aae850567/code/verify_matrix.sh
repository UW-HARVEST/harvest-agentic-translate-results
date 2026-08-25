#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
C_LIBRARY="$ROOT/c_src/build/libtranslated_rust.so"
RUST_LIBRARY="$ROOT/target/release/libencode_quant_lib.so"

mkdir -p "$ROOT/c_src/build"
cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build "$ROOT/c_src/build"

# Cargo.toml has no features, so the empty set is the complete matrix.
timeout 600 cargo fmt --manifest-path "$ROOT/Cargo.toml" --all -- --check
timeout 600 cargo check --manifest-path "$ROOT/Cargo.toml" --no-default-features
timeout 600 cargo build --manifest-path "$ROOT/Cargo.toml" --no-default-features
timeout 600 cargo test --manifest-path "$ROOT/Cargo.toml" --no-default-features
timeout 600 cargo build \
    --manifest-path "$ROOT/Cargo.toml" \
    --release \
    --no-default-features

missing_symbols="$(
    comm -23 \
        <(nm -D --defined-only --extern-only "$C_LIBRARY" |
            awk '{print $3}' | sort -u) \
        <(nm -D --defined-only --extern-only "$RUST_LIBRARY" |
            awk '{print $3}' | sort -u)
)"

if [[ -n "$missing_symbols" ]]; then
    printf 'Rust shared library is missing C symbols:\n%s\n' "$missing_symbols" >&2
    exit 1
fi

printf 'Feature matrix, differential tests, and symbol parity passed.\n'
