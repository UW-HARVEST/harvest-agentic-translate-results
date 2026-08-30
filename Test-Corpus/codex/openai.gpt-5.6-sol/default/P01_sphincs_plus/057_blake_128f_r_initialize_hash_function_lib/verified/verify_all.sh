#!/usr/bin/env bash
set -euo pipefail

crate_dir=$(cd "$(dirname "$0")" && pwd)
c_src_dir=$(cd "$crate_dir/../c_src" && pwd)
compat_dir="$crate_dir/c_compat"
build_root="$crate_dir/target/c-matrix"
start_combo=${START_COMBO:-}
started=false
if [[ -z "$start_combo" ]]; then
  started=true
fi

for hash in haraka sha2 shake blake; do
  for thash in robust simple; do
    for param in 128f 128s 192f 192s 256f 256s; do
      combo="$hash,$thash,$param"
      if [[ "$combo" == "$start_combo" ]]; then
        started=true
      fi
      if [[ "$started" != true ]]; then
        continue
      fi
      build_dir="$build_root/$hash-$thash-$param"
      timeout 600 cmake -S "$c_src_dir" -B "$build_dir" \
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="-I$compat_dir" \
        -DHASH_BACKEND="$hash" -DSECPAR="$param" -DTHASH="$thash"
      timeout 600 cmake --build "$build_dir" \
        --target "$hash" sphincs_core_det
      (
        cd "$crate_dir"
        timeout 600 cargo build --release --no-default-features --features "$combo"
        c_symbols=$(mktemp)
        rust_symbols=$(mktemp)
        missing_symbols=$(mktemp)
        nm -D --defined-only \
          "$build_dir/app/libsphincs_core_det.so" \
          "$build_dir/lib/$hash/lib$hash.so" |
          awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u > "$c_symbols"
        nm -D --defined-only target/release/libsphincs_plus_translation.so |
          awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u > "$rust_symbols"
        comm -23 "$c_symbols" "$rust_symbols" > "$missing_symbols"
        if [[ -s "$missing_symbols" ]]; then
          echo "Missing Rust exports for $combo:" >&2
          sed 's/^/  /' "$missing_symbols" >&2
          exit 1
        fi
        rm -f "$c_symbols" "$rust_symbols" "$missing_symbols"
        SPX_C_BUILD_DIR="$build_dir" \
          timeout 600 cargo test --release --no-default-features --features "$combo" \
            -- --test-threads=1
      )
    done
  done
done
