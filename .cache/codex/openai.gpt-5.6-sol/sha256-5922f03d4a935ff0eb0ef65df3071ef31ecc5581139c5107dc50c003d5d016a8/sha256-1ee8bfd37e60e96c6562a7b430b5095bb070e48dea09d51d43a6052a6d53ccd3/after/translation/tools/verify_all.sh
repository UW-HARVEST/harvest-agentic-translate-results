#!/bin/sh
set -eu

crate_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
c_so=$(find "$crate_dir/../c_src/build" -maxdepth 1 -type f -name '*.so' -print -quit)
rust_so="$crate_dir/target/release/libenvy_lib.so"

if [ -z "$c_so" ]; then
    echo "C shared library is missing; build c_src first." >&2
    exit 1
fi

cd "$crate_dir"
for mode in default no-default-features; do
    case "$mode" in
        default)
            feature_args=
            ;;
        no-default-features)
            feature_args=--no-default-features
            ;;
    esac

    # Word splitting is intentional: this crate has no named feature arguments.
    timeout 600 cargo build --release $feature_args
    timeout 600 cargo test $feature_args
done

c_symbols=$(mktemp)
rust_symbols=$(mktemp)
trap 'rm -f "$c_symbols" "$rust_symbols"' EXIT HUP INT TERM

nm -D --defined-only "$c_so" | awk '{ print $3 }' | sort -u > "$c_symbols"
nm -D --defined-only "$rust_so" | awk '{ print $3 }' | sort -u > "$rust_symbols"

if ! diff -u "$c_symbols" "$rust_symbols"; then
    echo "Exported symbol sets differ." >&2
    exit 1
fi
