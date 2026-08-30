#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

c_root="../c_src"
c_shared_dir="$c_root/build/shared"
rust_so="target/release/libmacrodepth_add_5.so"
mkdir -p "$c_shared_dir"

for op in add sub mul; do
    for repeat in 0 1 2 3 4 5 6 7; do
        features="$op,$repeat"
        c_so="$c_shared_dir/libmacrodepth_${op}_${repeat}.so"
        echo "== $features =="

        timeout 600 cc -shared -fPIC \
            -DOP="$op" -DREPEAT="$repeat" \
            "$c_root/src/mdcore.c" "$c_root/src/mdmain.c" \
            -o "$c_so"

        timeout 600 cargo check --no-default-features --features "$features"
        timeout 600 cargo build --release --no-default-features --features "$features"
        timeout 600 cargo test --no-default-features --features "$features" \
            -- --test-threads=1

        missing="$(
            comm -23 \
                <(nm -D --defined-only "$c_so" | awk '{print $3}' | sort) \
                <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort)
        )"
        if [[ -n "$missing" ]]; then
            printf 'Rust is missing C symbols for %s:\n%s\n' "$features" "$missing" >&2
            exit 1
        fi
    done
done
