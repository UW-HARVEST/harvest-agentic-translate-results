#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

timeout 600 cargo fmt --check

for args in "" "--no-default-features"; do
    read -r -a cargo_args <<< "$args"
    timeout 600 cargo check "${cargo_args[@]}"
    timeout 600 cargo build --release "${cargo_args[@]}"
    timeout 600 cargo test "${cargo_args[@]}"
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT

nm -D --defined-only ../c_src/build/libdriver.so |
    awk '{print $3}' |
    sort > "$c_symbols"
nm -D --defined-only target/release/libdriver.so |
    awk '{print $3}' |
    sort > "$rust_symbols"

if ! missing="$(comm -23 "$c_symbols" "$rust_symbols")"; then
    exit 1
fi
if [[ -n "$missing" ]]; then
    printf 'Missing Rust exports:\n%s\n' "$missing" >&2
    exit 1
fi

printf 'Verification complete: all C exports are present.\n'
