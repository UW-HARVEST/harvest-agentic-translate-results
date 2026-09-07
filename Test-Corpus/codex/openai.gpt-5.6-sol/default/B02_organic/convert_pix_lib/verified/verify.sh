#!/usr/bin/env bash
set -euo pipefail

timeout 600 cargo check
timeout 600 cargo build --release
timeout 600 cargo test --release -- --test-threads=1

timeout 600 cargo check --no-default-features
timeout 600 cargo build --release --no-default-features
timeout 600 cargo test --release --no-default-features -- --test-threads=1

c_so="../c_src/build/libharvest-work-63lrKy.so"
rust_so="target/release/libconvert_pix_lib.so"

missing_symbols="$(comm -23 \
  <(nm -D --defined-only "$c_so" | awk '{print $3}' | sort) \
  <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort))"

if [[ -n "$missing_symbols" ]]; then
  echo "Missing Rust exports:" >&2
  echo "$missing_symbols" >&2
  exit 1
fi
