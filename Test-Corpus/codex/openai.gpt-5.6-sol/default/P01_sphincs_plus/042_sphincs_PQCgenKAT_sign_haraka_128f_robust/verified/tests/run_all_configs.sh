#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
root_dir="$(cd "$crate_dir/.." && pwd)"
c_src="$root_dir/c_src"
node_root="$(cd "$(dirname "$(command -v node)")/.." && pwd)"
openssl_include="$node_root/include/node"
openssl_arch_include="$openssl_include/openssl/archs/linux-x86_64/asm/include"

for backend in blake haraka sha2 shake; do
  for thash in simple robust; do
    for secpar in 128s 128f 192s 192f 256s 256f; do
      combo="$backend,$thash,$secpar"
      build_dir="$c_src/build-matrix/$backend-$thash-$secpar"
      echo "=== $combo"
      cmake -S "$c_src" -B "$build_dir" \
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="-I$openssl_include -I$openssl_arch_include" \
        -DHASH_BACKEND="$backend" \
        -DSECPAR="$secpar" \
        -DTHASH="$thash" >/tmp/sphincs-cmake-matrix.log 2>&1
      timeout 600 cmake --build "$build_dir" --target "$backend" sphincs_core sphincs_core_det \
        >/tmp/sphincs-c-build-matrix.log 2>&1
      (
        cd "$crate_dir"
        timeout 600 cargo build --release --no-default-features --features "$combo" \
          >/tmp/sphincs-rust-build-matrix.log 2>&1
        SPHINCS_C_DIR="$build_dir" \
        SPHINCS_RUST_SO="$crate_dir/target/release/libsphincs_plus.so" \
        SPHINCS_C_DET_CORE="$build_dir/app/libsphincs_core_det.so" \
        SPHINCS_C_DET_BACKEND="$build_dir/lib/$backend/lib$backend.so" \
        timeout 600 cargo test --no-default-features --features "$combo" \
          --test differential -- --test-threads=1 \
          >/tmp/sphincs-test-matrix.log 2>&1
      )
    done
  done
done
