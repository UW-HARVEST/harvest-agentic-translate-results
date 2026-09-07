#!/usr/bin/env bash
set -euo pipefail

translation_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd "${translation_dir}/.." && pwd)"
c_build_dir="${project_dir}/c_src/build"
c_library="${c_build_dir}/libharvest-work-vRYfpk.so"
rust_library="${translation_dir}/target/release/libstr_put_lib.so"

mkdir -p "${c_build_dir}"
(
    cd "${c_build_dir}"
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    timeout 600 cmake --build .
)

cd "${translation_dir}"
for feature_state in default no-default-features; do
    if [[ "${feature_state}" == default ]]; then
        timeout 600 cargo check
        timeout 600 cargo build --release
        timeout 600 cargo test --release -- --test-threads=1
    else
        timeout 600 cargo check --no-default-features
        timeout 600 cargo build --release --no-default-features
        timeout 600 cargo test --release --no-default-features -- --test-threads=1
    fi
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
missing_symbols="$(mktemp)"
extra_symbols="$(mktemp)"
trap 'rm -f "${c_symbols}" "${rust_symbols}" "${missing_symbols}" "${extra_symbols}"' EXIT

nm -D --defined-only "${c_library}" |
    awk '$2 ~ /^[TDBR]$/ { print $3 }' |
    sort -u >"${c_symbols}"
nm -D --defined-only "${rust_library}" |
    awk '$2 ~ /^[TDBR]$/ { print $3 }' |
    sort -u >"${rust_symbols}"

comm -23 "${c_symbols}" "${rust_symbols}" >"${missing_symbols}"
comm -13 "${c_symbols}" "${rust_symbols}" >"${extra_symbols}"

if [[ -s "${missing_symbols}" ]]; then
    echo "C symbols missing from Rust:" >&2
    sed 's/^/  /' "${missing_symbols}" >&2
    exit 1
fi

if [[ -s "${extra_symbols}" ]]; then
    echo "Rust-only exported symbols:" >&2
    sed 's/^/  /' "${extra_symbols}" >&2
    exit 1
fi

timeout 600 ldd -r "${rust_library}" >/dev/null
echo "verification complete: tests pass in both feature states; symbol diff is empty"
