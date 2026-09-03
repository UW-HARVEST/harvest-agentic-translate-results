#!/usr/bin/env bash
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")" && pwd)"
c_dir="$crate_dir/../c_src"
c_lib_dir="$c_dir/build/ffi"
rust_lib_dir="$crate_dir/target/ffi"
mkdir -p "$c_lib_dir"
mkdir -p "$rust_lib_dir"

for op in add sub mul; do
    for repeat in 0 1 2 3 4 5 6 7; do
        timeout 600 cc -shared -fPIC -O0 \
            -DOP="$op" -DREPEAT="$repeat" \
            "$c_dir/src/mdcore.c" \
            -o "$c_lib_dir/libmdcore_${op}_${repeat}.so"
    done
done

cd "$crate_dir"

check_symbols() {
    local op="$1"
    local repeat="$2"
    local rust_library="$3"
    local c_symbols
    local rust_symbols
    local missing
    c_symbols="$(mktemp)"
    rust_symbols="$(mktemp)"
    missing="$(mktemp)"
    nm -D --defined-only --format=posix \
        "$c_lib_dir/libmdcore_${op}_${repeat}.so" |
        awk '{print $1}' | sort -u >"$c_symbols"
    nm -D --defined-only --format=posix \
        "$rust_library" |
        awk '{print $1}' | sort -u >"$rust_symbols"
    comm -23 "$c_symbols" "$rust_symbols" >"$missing"
    if [[ -s "$missing" ]]; then
        echo "Missing Rust symbols for $op,$repeat:" >&2
        sed 's/^/  /' "$missing" >&2
        rm -f "$c_symbols" "$rust_symbols" "$missing"
        return 1
    fi
    rm -f "$c_symbols" "$rust_symbols" "$missing"
}

run_configuration() {
    local expected_op="$1"
    local expected_repeat="$2"
    local features="$3"
    local label="${features:-default-empty}"
    local rust_library="$rust_lib_dir/libmd_driver_${expected_op}_${expected_repeat}.so"
    echo "VERIFY $label ($expected_op,$expected_repeat)"
    if [[ -n "$features" ]]; then
        timeout 600 cargo check --no-default-features --features "$features"
        timeout 600 cargo build --no-default-features --features "$features"
    else
        timeout 600 cargo check --no-default-features
        timeout 600 cargo build --no-default-features
    fi
    cp "$crate_dir/target/debug/libmd_driver.so" "$rust_library"
    check_symbols "$expected_op" "$expected_repeat" "$rust_library"
    if [[ -n "$features" ]]; then
        MD_RUST_SO="$rust_library" \
            timeout 600 cargo test --no-default-features --features "$features"
    else
        MD_RUST_SO="$rust_library" \
            timeout 600 cargo test --no-default-features
    fi
}

for op in add sub mul; do
    for repeat in 0 1 2 3 4 5 6 7; do
        run_configuration "$op" "$repeat" "$op,$repeat"
    done
done

run_configuration add 5 ""

for op in add sub mul; do
    run_configuration "$op" 5 "$op"
done

for repeat in 0 1 2 3 4 5 6 7; do
    run_configuration add "$repeat" "$repeat"
done
