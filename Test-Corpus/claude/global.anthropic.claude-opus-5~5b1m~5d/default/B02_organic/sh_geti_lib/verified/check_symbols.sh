#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
# Exits non-zero if the Rust .so is missing ANY symbol the C .so exports.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
c_so="$(ls -t "$here"/../c_src/build/lib*.so 2>/dev/null | head -1)"
rust_so="${1:-$here/target/release/libsh_geti_lib.so}"

if [[ -z "${c_so:-}" || ! -f "$c_so" ]]; then
  echo "FAIL: no C .so found under $here/../c_src/build" >&2
  exit 1
fi
if [[ ! -f "$rust_so" ]]; then
  echo "FAIL: no Rust .so at $rust_so" >&2
  exit 1
fi

tmp="${TMPDIR:-/tmp}"
nm -D --defined-only "$c_so"    | awk '$2=="T"{print $3}' | sort -u > "$tmp/c_syms.txt"
nm -D --defined-only "$rust_so" | awk '$2=="T"{print $3}' | sort -u > "$tmp/r_syms.txt"

echo "C    .so : $c_so   ($(wc -l < "$tmp/c_syms.txt") exported T symbols)"
echo "Rust .so : $rust_so ($(wc -l < "$tmp/r_syms.txt") exported T symbols)"

missing="$(comm -23 "$tmp/c_syms.txt" "$tmp/r_syms.txt")"
extra="$(comm -13 "$tmp/c_syms.txt" "$tmp/r_syms.txt")"

if [[ -n "$missing" ]]; then
  echo "FAIL: symbols exported by the C .so but MISSING from the Rust .so:"
  echo "$missing" | sed 's/^/  - /'
  exit 1
fi
echo "OK: 0 missing symbols."

if [[ -n "$extra" ]]; then
  echo "note: extra symbols exported only by the Rust .so:"
  echo "$extra" | sed 's/^/  + /'
fi

# Every imported symbol must be resolvable at load time.  `ldd -r` performs the
# real relocation check, which is authoritative (and does not depend on where
# libc lives on this distro).
echo
echo "Unresolved (undefined) symbols per \`ldd -r\`:"
unres_r="$(ldd -r "$rust_so" 2>&1 | grep -i "undefined symbol" || true)"
unres_c="$(ldd -r "$c_so"    2>&1 | grep -i "undefined symbol" || true)"
if [[ -n "$unres_r" ]]; then
  echo "FAIL: Rust .so has unresolved symbols:"
  echo "$unres_r" | sed 's/^/  /'
  exit 1
fi
echo "  Rust .so: none"
if [[ -n "$unres_c" ]]; then
  echo "  C .so   : $unres_c"
else
  echo "  C .so   : none"
fi

# Also show, for the record, the non-libc imports of each .so.
echo
echo "Imported symbol names (for the record):"
echo "  C    : $(nm -D --undefined-only "$c_so"    | awk '{print $NF}' | sed 's/@.*//' | sort -u | tr '\n' ' ')"
echo "  Rust : $(nm -D --undefined-only "$rust_so" | awk '{print $NF}' | sed 's/@.*//' | sort -u | tr '\n' ' ')"
echo
echo "SYMBOL PARITY: OK"
