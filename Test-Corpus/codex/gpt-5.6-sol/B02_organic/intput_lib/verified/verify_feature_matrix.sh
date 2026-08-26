#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$crate_dir"

timeout 600 sh -c '
  mkdir -p c_src/build
  cd c_src/build
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
  cmake --build .
'

# Cargo.toml declares no features, so the empty set is the full matrix.
feature_combinations=("")
for features in "${feature_combinations[@]}"; do
  cargo_args=(--no-default-features)
  if [[ -n "$features" ]]; then
    cargo_args+=(--features "$features")
  fi

  timeout 600 cargo check "${cargo_args[@]}"
  timeout 600 cargo build "${cargo_args[@]}"
  timeout 600 cargo test "${cargo_args[@]}" -- --test-threads=1
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT

nm -D --defined-only c_src/build/libtranslated_rust.so |
  awk '{print $3}' |
  sort -u >"$c_symbols"
nm -D --defined-only target/debug/libintput_lib.so |
  awk '{print $3}' |
  sort -u >"$rust_symbols"

if ! diff -u "$c_symbols" "$rust_symbols"; then
  echo "dynamic symbol mismatch" >&2
  exit 1
fi
