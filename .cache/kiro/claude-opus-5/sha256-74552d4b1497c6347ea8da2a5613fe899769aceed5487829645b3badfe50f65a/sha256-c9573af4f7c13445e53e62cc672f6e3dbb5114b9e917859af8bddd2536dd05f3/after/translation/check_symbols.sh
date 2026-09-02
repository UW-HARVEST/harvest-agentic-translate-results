#!/usr/bin/env bash
# Phase D: exported-symbol parity between the C .so and the Rust .so.
# Exits non-zero if the Rust .so is missing any symbol the C .so exports.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

c_so="$(ls "$root"/c_src/build/lib*.so 2>/dev/null | head -n1)"
if [[ -z "${c_so:-}" ]]; then
    echo "FAIL: C .so not built. Run:"
    echo "  cd $root/c_src && mkdir -p build && cd build && \\"
    echo "    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi

rs_so=""
for p in "$here/target/release/libhelxo_lib.so" "$here/target/debug/libhelxo_lib.so"; do
    [[ -f "$p" ]] && rs_so="$p" && break
done
if [[ -z "$rs_so" ]]; then
    echo "FAIL: Rust cdylib not built. Run: cd $here && cargo build --release"
    exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
nm -D --defined-only "$c_so"  | awk '{print $3}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$rs_so" | awk '{print $3}' | sort -u > "$tmp/r.txt"

echo "C   .so: $c_so  ($(wc -l < "$tmp/c.txt") defined symbols)"
echo "Rust.so: $rs_so ($(wc -l < "$tmp/r.txt") defined symbols)"

missing="$(comm -23 "$tmp/c.txt" "$tmp/r.txt")"
extra="$(comm -13 "$tmp/c.txt" "$tmp/r.txt")"

if [[ -n "$missing" ]]; then
    echo
    echo "FAIL: symbols exported by the C .so but MISSING from the Rust .so:"
    echo "$missing" | sed 's/^/  - /'
    exit 1
fi

echo
echo "OK: 0 missing symbols (symbol diff is empty)."
if [[ -n "$extra" ]]; then
    echo "note: Rust-only symbols (allowed; not a parity violation):"
    echo "$extra" | sed 's/^/  + /'
fi

# C `static` definitions must not be exported by either library.
for s in stbds_probe_position stbds_log2 stbds_make_hash_index stbds_siphash_bytes \
         stbds_is_key_equal stbds_hm_find_slot stbds_strdup stbds_hash_seed buffer; do
    if grep -qx "$s" "$tmp/r.txt"; then
        echo "FAIL: Rust .so exports '$s', which is \`static\` in the C."
        exit 1
    fi
done
echo "OK: no C-private symbol is exported by the Rust .so."
