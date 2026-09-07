#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
c_dir="$(cd "${crate_dir}/../c_src" && pwd)"
c_so="${c_dir}/build/libharvest-work-i1tAuE.so"
rust_so="${crate_dir}/target/release/libsh_puts_lib.so"

mkdir -p "${c_dir}/build"
(
    cd "${c_dir}/build"
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    timeout 600 cmake --build .
)

run_combo() {
    local label="$1"
    shift
    printf 'Verifying feature configuration: %s\n' "${label}"
    (
        cd "${crate_dir}"
        timeout 600 cargo check "$@"
        timeout 600 cargo build --release "$@"
        RUST_TRANSLATION_SO="${rust_so}" \
            timeout 600 cargo test "$@" -- --test-threads=1
    )
}

run_combo default
run_combo no-default-features --no-default-features

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
trap 'rm -f "${c_symbols}" "${rust_symbols}"' EXIT

nm -D --defined-only "${c_so}" |
    awk '$2 ~ /^[TDBRWV]$/ {print $3}' |
    sort -u >"${c_symbols}"
nm -D --defined-only "${rust_so}" |
    awk '$2 ~ /^[TDBRWV]$/ {print $3}' |
    sort -u >"${rust_symbols}"

if ! diff -u "${c_symbols}" "${rust_symbols}"; then
    printf 'Exported symbol mismatch\n' >&2
    exit 1
fi

printf 'All feature configurations and exported symbols match.\n'
