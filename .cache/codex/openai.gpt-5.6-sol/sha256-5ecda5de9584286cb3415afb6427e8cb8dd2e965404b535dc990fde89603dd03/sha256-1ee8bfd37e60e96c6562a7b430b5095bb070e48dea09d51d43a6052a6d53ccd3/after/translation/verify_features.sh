#!/bin/sh
set -eu

cd "$(dirname "$0")"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

for configuration in default no-default-features; do
    case "$configuration" in
        default)
            feature_args=""
            ;;
        no-default-features)
            feature_args="--no-default-features"
            ;;
    esac

    echo "==> $configuration"
    # shellcheck disable=SC2086
    timeout 600 cargo check $feature_args
    # shellcheck disable=SC2086
    timeout 600 cargo build --release $feature_args
    # shellcheck disable=SC2086
    timeout 600 cargo test $feature_args -- --test-threads=1

    nm -D --defined-only ../c_src/build/libharvest-work-v0LDLj.so \
        | awk '{print $3}' | sort -u > "$tmpdir/c-symbols"
    nm -D --defined-only target/release/libdequantize_granule_lib.so \
        | awk '{print $3}' | sort -u > "$tmpdir/rust-symbols"
    diff -u "$tmpdir/c-symbols" "$tmpdir/rust-symbols"
done
