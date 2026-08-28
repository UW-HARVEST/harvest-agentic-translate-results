#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
# Exits non-zero if the Rust .so is missing any symbol the C .so exports.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
c_so="$(ls "$here"/../c_src/build/lib*.so 2>/dev/null | head -1)"
rust_so="${1:-$here/target/release/libima_parse_lib.so}"

if [[ -z "$c_so" || ! -f "$c_so" ]]; then
    echo "FAIL: C .so not found. Build it with:" >&2
    echo "  cd c_src && mkdir -p build && cd build && \\" >&2
    echo "    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
    exit 1
fi
if [[ ! -f "$rust_so" ]]; then
    echo "FAIL: Rust .so not found at $rust_so (run: cargo build --release)" >&2
    exit 1
fi

echo "C    .so: $c_so"
echo "Rust .so: $rust_so"
echo

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$c_so"    | awk '{print $NF}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u > "$tmp/r.txt"

echo "--- C exported dynamic symbols ($(wc -l < "$tmp/c.txt")) ---"
cat "$tmp/c.txt"
echo
echo "--- Rust exported dynamic symbols ($(wc -l < "$tmp/r.txt")) ---"
cat "$tmp/r.txt"
echo

missing="$(comm -23 "$tmp/c.txt" "$tmp/r.txt")"
if [[ -n "$missing" ]]; then
    echo "--- MISSING from the Rust .so ---"
    echo "$missing"
    echo
    echo "FAIL: $(echo "$missing" | wc -l) symbol(s) exported by C but not by Rust."
    exit 1
fi
echo "OK: 0 symbols missing from the Rust .so."

# Undefined symbols in the Rust .so must all be libc / libgcc imports.
echo
echo "--- Rust undefined symbols that are NOT libc/libgcc ---"
nm -D --undefined-only "$rust_so" | awk '{print $NF}' | sort -u \
  | grep -vE '@GLIBC_|@GCC_|^_ITM_|^__gmon_start__$|^_Unwind_|^statx$|^gettid$|^__cxa_' \
  > "$tmp/undef.txt" || true
if [[ -s "$tmp/undef.txt" ]]; then
    cat "$tmp/undef.txt"
    echo "FAIL: unresolved non-libc symbols in the Rust .so."
    exit 1
fi
echo "(none)"
echo
echo "OK: symbol parity verified."
