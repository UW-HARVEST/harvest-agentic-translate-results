#!/usr/bin/env bash
set -euo pipefail

crate_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
c_root="$(cd "${crate_root}/../c_src" && pwd)"
c_so="${c_root}/build/libharvest-work-OooqjH.so"
rust_so="${crate_root}/target/release/libaabb_lib.so"

timeout 600 cmake -S "${c_root}" -B "${c_root}/build" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
timeout 600 cmake --build "${c_root}/build"

cd "${crate_root}"

# Cargo.toml declares no named features, so these are all feature configurations.
for mode in default no-default-features; do
    cargo_args=()
    if [[ "${mode}" == no-default-features ]]; then
        cargo_args+=(--no-default-features)
    fi

    echo "==> checking ${mode}"
    timeout 600 cargo check "${cargo_args[@]}"
    echo "==> building release cdylib for ${mode}"
    timeout 600 cargo build --release "${cargo_args[@]}"
    echo "==> testing ${mode}"
    timeout 600 cargo test "${cargo_args[@]}" --no-fail-fast
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
missing_symbols="$(mktemp)"
trap 'rm -f "${c_symbols}" "${rust_symbols}" "${missing_symbols}"' EXIT

nm -D --defined-only "${c_so}" | awk '{print $3}' | sort >"${c_symbols}"
nm -D --defined-only "${rust_so}" | awk '{print $3}' | sort >"${rust_symbols}"
comm -23 "${c_symbols}" "${rust_symbols}" >"${missing_symbols}"

if [[ -s "${missing_symbols}" ]]; then
    echo "Rust shared object is missing C symbols:" >&2
    cat "${missing_symbols}" >&2
    exit 1
fi

echo "==> symbol parity: $(wc -l <"${c_symbols}") C exports, 0 missing in Rust"
