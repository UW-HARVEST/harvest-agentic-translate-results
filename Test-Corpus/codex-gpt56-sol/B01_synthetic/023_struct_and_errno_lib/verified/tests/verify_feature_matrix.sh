#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

timeout 600 cmake \
    -S c_src \
    -B c_src/build \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build

# Cargo.toml defines no features, so the empty set is the complete matrix.
feature_sets=("")
for features in "${feature_sets[@]}"; do
    cargo_args=(--no-default-features)
    if [[ -n "$features" ]]; then
        cargo_args+=(--features "$features")
    fi

    timeout 600 cargo check "${cargo_args[@]}"
    timeout 600 cargo build --release "${cargo_args[@]}"
    DRIVER_RUST_SO="$root/target/release/libdriver.so" \
        timeout 600 cargo test "${cargo_args[@]}" -- --test-threads=1
done

missing_symbols="$(
    comm -23 \
        <(nm -D --defined-only c_src/build/libdriver.so |
            awk '{print $3}' | sort -u) \
        <(nm -D --defined-only target/release/libdriver.so |
            awk '{print $3}' | sort -u)
)"
if [[ -n "$missing_symbols" ]]; then
    printf 'Rust shared library is missing C exports:\n%s\n' "$missing_symbols" >&2
    exit 1
fi
