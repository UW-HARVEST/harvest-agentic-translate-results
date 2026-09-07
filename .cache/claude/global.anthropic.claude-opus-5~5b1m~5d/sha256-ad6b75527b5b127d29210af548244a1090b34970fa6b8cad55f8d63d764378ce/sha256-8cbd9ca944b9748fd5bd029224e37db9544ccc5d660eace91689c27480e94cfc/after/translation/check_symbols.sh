#!/bin/sh
# Phase A / Phase D: symbol parity between the C .so and the Rust .so.
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)

cso=$(ls "$root"/c_src/build/lib*.so 2>/dev/null | head -1)
rso="$here/target/release/libpinflate_lib.so"

[ -f "$cso" ] || { echo "C .so not built: run cmake in c_src/build"; exit 1; }
[ -f "$rso" ] || { echo "Rust .so not built: cargo build --release"; exit 1; }

tmp=${TMPDIR:-/tmp}
nm -D --defined-only "$cso" | awk '{print $3}' | sort -u > "$tmp/c_syms"
nm -D --defined-only "$rso" | awk '{print $3}' | sort -u > "$tmp/r_syms"

echo "C exports  : $(wc -l < "$tmp/c_syms")"
echo "Rust exports: $(wc -l < "$tmp/r_syms")"

missing=$(comm -23 "$tmp/c_syms" "$tmp/r_syms")
if [ -n "$missing" ]; then
    echo "MISSING FROM RUST:"
    echo "$missing"
    exit 1
fi
echo "symbol diff (C -> Rust): EMPTY  ✓"

extra=$(comm -13 "$tmp/c_syms" "$tmp/r_syms")
[ -n "$extra" ] && { echo "extra in Rust (informational):"; echo "$extra"; }

# Undefined non-libc symbols in the Rust .so
echo "--- Rust undefined ---"
nm -D -u "$rso"
exit 0
