#!/bin/sh
set -eu

cd "$(dirname "$0")"

c_so="../c_src/build/libmujs.so"
rust_so="target/release/libmujs.so"

test -f "$c_so"

timeout 600 cargo build --release

for mode in default no-default-features
do
    case "$mode" in
        default)
            timeout 600 cargo check
            timeout 600 cargo test
            ;;
        no-default-features)
            timeout 600 cargo check --no-default-features
            timeout 600 cargo test --no-default-features
            ;;
    esac
done

c_symbols="$(mktemp)"
rust_symbols="$(mktemp)"
missing_symbols="$(mktemp)"
trap 'rm -f "$c_symbols" "$rust_symbols" "$missing_symbols"' EXIT HUP INT TERM

nm -D --defined-only --format=posix "$c_so" |
    awk '{print $1}' |
    sort -u >"$c_symbols"
nm -D --defined-only --format=posix "$rust_so" |
    awk '{print $1}' |
    sort -u >"$rust_symbols"
comm -23 "$c_symbols" "$rust_symbols" >"$missing_symbols"

test "$(wc -l <"$c_symbols")" -eq 237
test ! -s "$missing_symbols"
test "$(grep -c '^| [0-9][0-9]* | `[^`][^`]*` | present |$' SYMBOLS.md)" -eq 237
test "$(grep -c '^| [0-9][0-9]* |' ERRORS.md)" -eq 129
test "$(grep -c '^| [0-9][0-9]* |' CONFIGS.md)" -eq 35

echo "verification complete: 237 symbols, 129 error rows, 35 configuration rows"
