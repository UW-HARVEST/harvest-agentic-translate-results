#!/usr/bin/env bash
# Regenerate the SYMBOLS.md symbol diff. Exits non-zero if the Rust .so is
# missing any symbol that the C .so exports.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"

c_build="$root/c_src/build"
if [ ! -d "$c_build" ]; then
    echo "building the C library first..."
    ( cd "$root/c_src" && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null ) || exit 1
fi

c_so="$(find "$c_build" -maxdepth 1 -name '*.so' | head -1)"
rust_so="$here/target/release/libcomplexmode_lib.so"
[ -f "$rust_so" ] || rust_so="$here/target/debug/libcomplexmode_lib.so"

if [ -z "$c_so" ] || [ ! -f "$rust_so" ]; then
    echo "FAIL: missing .so (c='$c_so' rust='$rust_so')"
    exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
nm -D --defined-only "$c_so"    | awk '{print $NF}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u > "$tmp/r.txt"

echo "C    .so: $c_so   ($(wc -l < "$tmp/c.txt") exported symbols)"
echo "Rust .so: $rust_so   ($(wc -l < "$tmp/r.txt") exported symbols)"
echo
echo "--- exported by C but NOT by Rust (must be empty) ---"
comm -23 "$tmp/c.txt" "$tmp/r.txt" | tee "$tmp/missing.txt"
echo "--- exported by Rust but NOT by C ---"
comm -13 "$tmp/c.txt" "$tmp/r.txt" | tee "$tmp/extra.txt"
echo
echo "--- Rust undefined (imported) symbols ---"
nm -D --undefined-only "$rust_so" | awk '{print $NF}' | sort -u

missing=$(wc -l < "$tmp/missing.txt")
extra=$(wc -l < "$tmp/extra.txt")
echo
if [ "$missing" -eq 0 ] && [ "$extra" -eq 0 ]; then
    echo "PASS: symbol sets are identical"
    exit 0
fi
echo "FAIL: missing=$missing extra=$extra"
exit 1
