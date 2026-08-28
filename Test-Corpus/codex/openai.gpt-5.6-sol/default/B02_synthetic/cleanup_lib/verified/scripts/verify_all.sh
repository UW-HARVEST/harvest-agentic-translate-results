#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

# Cargo.toml declares no features. Default and no-default-features are the two
# command modes, and both represent the same sole feature configuration.
for mode in default no-default-features; do
    if test "$mode" = default; then
        feature_args=
    else
        feature_args=--no-default-features
    fi

    timeout 600 cargo check $feature_args
    timeout 600 cargo build --release $feature_args
    timeout 600 cargo test $feature_args
done

c_library=../c_src/build/libharvest-work-KQ5axu.so
rust_library=target/release/libcleanup_lib.so
test -f "$c_library"
test -f "$rust_library"

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT HUP INT TERM

nm -D --defined-only "$c_library" |
    awk '$2 ~ /^[A-Z]$/ { print $3 }' |
    sort -u >"$temporary_directory/c-symbols"
nm -D --defined-only "$rust_library" |
    awk '$2 ~ /^[A-Z]$/ { print $3 }' |
    sort -u >"$temporary_directory/rust-symbols"

comm -23 "$temporary_directory/c-symbols" "$temporary_directory/rust-symbols" \
    >"$temporary_directory/missing-symbols"
test ! -s "$temporary_directory/missing-symbols"

if ldd -r "$rust_library" 2>&1 | grep -q 'undefined symbol:'; then
    ldd -r "$rust_library"
    exit 1
fi

printf 'C API symbols: %s\n' "$(wc -l <"$temporary_directory/c-symbols")"
printf 'Missing Rust symbols: 0\n'
printf 'Unresolved Rust dynamic symbols: 0\n'
