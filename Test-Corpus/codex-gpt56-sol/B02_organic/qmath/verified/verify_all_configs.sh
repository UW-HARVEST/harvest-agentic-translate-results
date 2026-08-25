#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Cargo.toml has no [features] table, so the powerset has one member.
feature_combinations=("")

mkdir -p c_src/build
(
    cd c_src/build
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    timeout 600 cmake --build .
)
timeout 600 cc -shared -fPIC -I c_src/inc c_src/src/q_math.c -lm \
    -o c_src/build/libqmath_c.so

for features in "${feature_combinations[@]}"; do
    cargo_args=(--no-default-features)
    if [[ -n "$features" ]]; then
        cargo_args+=(--features "$features")
    fi

    timeout 600 cargo check "${cargo_args[@]}"
    # cargo test builds the rlib test artifact, so build the loadable cdylib first.
    timeout 600 cargo build "${cargo_args[@]}"
    timeout 600 cargo test "${cargo_args[@]}"

    c_symbols="$(mktemp)"
    rust_symbols="$(mktemp)"
    nm -D --defined-only c_src/build/libqmath_c.so | awk '{print $3}' | sort > "$c_symbols"
    nm -D --defined-only target/debug/libqmath.so | awk '{print $3}' | sort > "$rust_symbols"
    if ! diff -u "$c_symbols" <(comm -12 "$c_symbols" "$rust_symbols"); then
        echo "Rust shared object is missing C symbols" >&2
        rm -f "$c_symbols" "$rust_symbols"
        exit 1
    fi
    rm -f "$c_symbols" "$rust_symbols"
done
