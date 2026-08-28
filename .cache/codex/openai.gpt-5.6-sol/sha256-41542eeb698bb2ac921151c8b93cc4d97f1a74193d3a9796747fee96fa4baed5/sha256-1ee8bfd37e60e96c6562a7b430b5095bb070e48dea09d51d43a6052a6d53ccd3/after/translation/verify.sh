#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

run_combo() {
    local label="$1"
    shift
    printf 'Verifying feature combination: %s\n' "$label"
    timeout 600 cargo check "$@"
    timeout 600 cargo build --release "$@"
    timeout 600 cargo test "$@" -- --test-threads=1
}

run_combo default
run_combo no-default-features --no-default-features

c_symbols="$(
    nm -D --defined-only ../c_src/build/*.so |
        awk '$2 ~ /^[TDBRWV]$/ {print $3}' |
        sort -u
)"
rust_symbols="$(
    nm -D --defined-only target/release/libdoubleneg_lib.so |
        awk '$2 ~ /^[TDBRWV]$/ {print $3}' |
        sort -u
)"
missing="$(comm -23 <(printf '%s\n' "$c_symbols") <(printf '%s\n' "$rust_symbols"))"

if [[ -n "$missing" ]]; then
    printf 'C symbols missing from Rust:\n%s\n' "$missing" >&2
    exit 1
fi

printf 'Symbol parity passed: %s C exports, 0 missing from Rust.\n' \
    "$(printf '%s\n' "$c_symbols" | wc -l)"
