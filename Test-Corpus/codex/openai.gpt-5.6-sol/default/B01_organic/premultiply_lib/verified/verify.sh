#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"

c_build_dir="../c_src/build"
c_library="$c_build_dir/libharvest-work-9Toehl.so"
rust_library="target/release/libpremultiply_lib.so"

mkdir -p "$c_build_dir"
(
    cd "$c_build_dir"
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    timeout 600 cmake --build .
)

run_configuration() {
    local -a cargo_args=("$@")
    timeout 600 cargo check "${cargo_args[@]}"
    timeout 600 cargo build --release "${cargo_args[@]}"
    timeout 600 cargo test "${cargo_args[@]}" -- --test-threads=1
}

run_configuration
run_configuration --no-default-features

missing_symbols="$(
    comm -23 \
        <(nm -D --defined-only "$c_library" | awk '{print $3}' | sort -u) \
        <(nm -D --defined-only "$rust_library" | awk '{print $3}' | sort -u)
)"

if [[ -n "$missing_symbols" ]]; then
    echo "Rust shared library is missing C exports:" >&2
    echo "$missing_symbols" >&2
    exit 1
fi

echo "verification complete: tests pass and symbol parity is exact"
