#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$crate_dir"

if grep -q '^\[features\]' Cargo.toml; then
    echo "Cargo.toml now declares features; enumerate their combinations here." >&2
    exit 1
fi

candidates=("$crate_dir"/../c_src/build/*.so)
if [[ ${#candidates[@]} -ne 1 || ! -f "${candidates[0]}" ]]; then
    echo "Expected exactly one built C shared library in ../c_src/build." >&2
    exit 1
fi
c_so="${candidates[0]}"

configurations=(default no-default-features)
for configuration in "${configurations[@]}"; do
    cargo_args=()
    if [[ "$configuration" == no-default-features ]]; then
        cargo_args+=(--no-default-features)
    fi

    timeout 600 cargo check "${cargo_args[@]}"
    timeout 600 cargo build --release "${cargo_args[@]}"
    timeout 600 cargo test --release "${cargo_args[@]}" -- --test-threads=1
done

rust_so="$crate_dir/target/release/libmatrixsum_lib.so"
test -f "$rust_so"

c_exports="$(mktemp)"
rust_exports="$(mktemp)"
symbol_diff="$(mktemp)"
trap 'rm -f "$c_exports" "$rust_exports" "$symbol_diff"' EXIT

nm -D --defined-only "$c_so" |
    awk '$2 ~ /^[TDBR]$/ { print $3 }' |
    sort -u >"$c_exports"
nm -D --defined-only "$rust_so" |
    awk '$2 ~ /^[TDBR]$/ { print $3 }' |
    sort -u >"$rust_exports"

comm -23 "$c_exports" "$rust_exports" >"$symbol_diff"
if [[ -s "$symbol_diff" ]]; then
    echo "C exports missing from Rust:" >&2
    cat "$symbol_diff" >&2
    exit 1
fi

if ldd -r "$rust_so" 2>&1 | grep -q 'undefined symbol'; then
    ldd -r "$rust_so" >&2
    exit 1
fi

echo "Verification passed for default and no-default-features configurations."
echo "Missing C exports in Rust: 0"
