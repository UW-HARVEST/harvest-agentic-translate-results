#!/usr/bin/env bash
# Phase B extra: drive BOTH .so files from a real gcc-compiled C consumer that
# uses the declared `void driver(char)` prototype, and diff stdout byte-for-byte
# over every one of the 256 char values.
#
# This is the "binary executable" comparison. c_src/CMakeLists.txt builds no
# executable of its own (only `add_library(driver SHARED ...)`), so this harness
# supplies an equivalent driver program and links it against each library in
# turn.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/.." && pwd)"
root="$(cd "$crate/.." && pwd)"

c_so="${C_DRIVER_SO:-$root/c_src/build/libdriver.so}"
rust_so="${RUST_DRIVER_SO:-$crate/target/release/libdriver.so}"
[[ -f "$c_so" ]] || { echo "missing C .so: $c_so" >&2; exit 1; }
[[ -f "$rust_so" ]] || { echo "missing Rust .so: $rust_so" >&2; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Link the same source against each library. -Wl,-rpath pins the exact .so.
gcc -O2 -Wall -Wextra -I"$root/c_src/include" \
    -o "$tmp/main_c" "$crate/tests/harness/driver_main.c" \
    -L"$(dirname "$c_so")" -l:"$(basename "$c_so")" \
    -Wl,-rpath,"$(dirname "$c_so")"

gcc -O2 -Wall -Wextra -I"$root/c_src/include" \
    -o "$tmp/main_rust" "$crate/tests/harness/driver_main.c" \
    -L"$(dirname "$rust_so")" -l:"$(basename "$rust_so")" \
    -Wl,-rpath,"$(dirname "$rust_so")"

fail=0
for v in $(seq -128 127); do
    "$tmp/main_c"    "$v" > "$tmp/out_c.bin"    2>"$tmp/err_c"
    "$tmp/main_rust" "$v" > "$tmp/out_rust.bin" 2>"$tmp/err_rust"
    if ! cmp -s "$tmp/out_c.bin" "$tmp/out_rust.bin"; then
        echo "DIVERGENCE for char value $v:" >&2
        echo "--- C ---"    >&2; cat -v "$tmp/out_c.bin"    >&2
        echo "--- Rust ---" >&2; cat -v "$tmp/out_rust.bin" >&2
        fail=1
    fi
    if [[ ! -s "$tmp/out_c.bin" ]]; then
        echo "C binary produced no output for $v" >&2
        fail=1
    fi
done

# Also feed the driver values the C compiler must narrow itself.
for v in 255 256 -129 4660 1000; do
    "$tmp/main_c"    "$v" > "$tmp/out_c.bin"
    "$tmp/main_rust" "$v" > "$tmp/out_rust.bin"
    if ! cmp -s "$tmp/out_c.bin" "$tmp/out_rust.bin"; then
        echo "DIVERGENCE for narrowed value $v" >&2
        fail=1
    fi
done

if [[ $fail -eq 0 ]]; then
    echo "binary stdout comparison: OK (256 char values + 5 narrowed values, byte-identical)"
else
    echo "binary stdout comparison: FAILED" >&2
    exit 1
fi
