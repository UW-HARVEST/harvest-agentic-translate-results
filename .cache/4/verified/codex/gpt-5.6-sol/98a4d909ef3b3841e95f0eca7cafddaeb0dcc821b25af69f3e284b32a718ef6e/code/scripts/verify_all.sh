#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

timeout 600 cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build
timeout 600 cc -fPIC -shared -Ic_src/include \
    c_src/src/scene.c c_src/src/shape.c -o c_src/build/libdriver_c.so

# Cargo.toml declares no features, so the complete combination matrix has one row.
timeout 600 cargo check --no-default-features
timeout 600 cargo build --release --no-default-features
timeout 600 cargo test --no-default-features -- --test-threads=1

C_SYMBOLS=$(mktemp)
RUST_SYMBOLS=$(mktemp)
trap 'rm -f "$C_SYMBOLS" "$RUST_SYMBOLS"' EXIT
nm -D --defined-only c_src/build/libdriver_c.so |
    awk '$2 ~ /^[TWDBR]$/ {print $3}' | sort >"$C_SYMBOLS"
nm -D --defined-only target/release/libdriver.so |
    awk '$2 ~ /^[TWDBR]$/ {print $3}' | sort >"$RUST_SYMBOLS"

if comm -23 "$C_SYMBOLS" "$RUST_SYMBOLS" | grep .; then
    echo "Rust shared library is missing C symbols" >&2
    exit 1
fi
