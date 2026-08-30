#!/usr/bin/env bash
set -euo pipefail

crate_dir=$(cd "$(dirname "$0")" && pwd)
build_root="$crate_dir/target/c-matrix"

for hash in haraka sha2 shake blake; do
  for thash in robust simple; do
    for param in 128f 128s 192f 192s 256f 256s; do
      combo="$hash,$thash,$param"
      build_dir="$build_root/$hash-$thash-$param"
      (
        cd "$crate_dir"
        timeout 600 cargo build --release --no-default-features --features "$combo"
        SPX_C_BUILD_DIR="$build_dir" \
          timeout 600 cargo test --release --no-default-features --features "$combo" \
            --test differential -- --test-threads=1
      )
    done
  done
done
