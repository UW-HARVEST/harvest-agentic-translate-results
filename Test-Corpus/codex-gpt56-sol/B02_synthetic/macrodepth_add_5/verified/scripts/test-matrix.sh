#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

ops=("" add sub mul)
repeats=("" 0 1 2 3 4 5 6 7)
count=0

for op_feature in "${ops[@]}"; do
    for repeat_feature in "${repeats[@]}"; do
        op="${op_feature:-add}"
        repeat="${repeat_feature:-5}"
        features="${op_feature}${op_feature:+${repeat_feature:+,}}${repeat_feature}"
        count=$((count + 1))
        printf '[%02d/36] features=%s C(OP=%s,REPEAT=%s)\n' \
            "$count" "${features:-<empty>}" "$op" "$repeat"

        cc -shared -fPIC -DOP="$op" -DREPEAT="$repeat" \
            c_src/src/mdcore.c c_src/src/mdmain.c \
            -o c_src/build/libdriver_c.so
        timeout 600 cargo build --lib --no-default-features --features "$features"
        C_REFERENCE_LIB="$root/c_src/build/libdriver_c.so" \
            RUST_TRANSLATION_LIB="$root/target/debug/libdriver.so" \
            timeout 600 cargo test --no-default-features --features "$features" \
                --test ffi_diff -- --test-threads=1
    done
done
