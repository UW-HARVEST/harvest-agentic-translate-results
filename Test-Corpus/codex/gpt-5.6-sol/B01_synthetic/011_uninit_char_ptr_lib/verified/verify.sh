#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$root"

timeout 600 cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build c_src/build

feature_combinations=("")
for features in "${feature_combinations[@]}"; do
    args=(--no-default-features)
    if [[ -n "$features" ]]; then
        args+=(--features "$features")
    fi

    timeout 600 cargo check "${args[@]}"
    timeout 600 cargo rustc --lib "${args[@]}"
    timeout 600 cargo test "${args[@]}"
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT

nm -D --defined-only c_src/build/libdriver.so \
    | awk '$2 ~ /^[TW]$/ {print $3}' \
    | sort -u >"$c_symbols"
nm -D --defined-only target/debug/libdriver.so \
    | awk '$2 ~ /^[TW]$/ {print $3}' \
    | sort -u >"$rust_symbols"

missing="$(comm -23 "$c_symbols" "$rust_symbols")"
if [[ -n "$missing" ]]; then
    printf 'Missing Rust exports:\n%s\n' "$missing" >&2
    exit 1
fi

printf 'Verified %d feature combination and %d C exports.\n' \
    "${#feature_combinations[@]}" "$(wc -l <"$c_symbols")"
