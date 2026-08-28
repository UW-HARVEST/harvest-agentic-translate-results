#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root_dir="$(cd "$crate_dir/.." && pwd)"
c_dir="$root_dir/c_src"
c_so="$c_dir/build/libharvest-work-m0JAPI.so"
rust_so="$crate_dir/target/release/libgjk_lib.so"

mkdir -p "$c_dir/build"
(
    cd "$c_dir/build"
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    timeout 600 cmake --build .
)

cd "$crate_dir"
timeout 600 cargo check --all-targets
timeout 600 cargo check --all-targets --no-default-features
timeout 600 cargo build --release
GJK_RUST_SO="$rust_so" timeout 600 cargo test
timeout 600 cargo test --no-default-features

missing="$(
    comm -23 \
        <(nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u) \
        <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u)
)"
if [[ -n "$missing" ]]; then
    printf 'Rust library is missing C exports:\n%s\n' "$missing" >&2
    exit 1
fi

bash tools/generate_surface_docs.sh --checked
