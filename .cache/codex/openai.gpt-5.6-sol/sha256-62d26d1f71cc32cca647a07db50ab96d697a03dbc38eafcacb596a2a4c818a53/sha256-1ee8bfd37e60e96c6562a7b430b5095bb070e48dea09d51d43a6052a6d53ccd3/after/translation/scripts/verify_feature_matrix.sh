#!/bin/sh
set -eu

if grep -q '^\[features\]' Cargo.toml; then
    echo "named features detected; extend this matrix generator" >&2
    exit 1
fi

for mode in default no-default-features; do
    echo "== $mode =="
    if [ "$mode" = default ]; then
        timeout 600 cargo check --tests
        timeout 600 cargo build --release
        timeout 600 cargo test -- --test-threads=1
    else
        timeout 600 cargo check --tests --no-default-features
        timeout 600 cargo build --release --no-default-features
        timeout 600 cargo test --no-default-features -- --test-threads=1
    fi
done
