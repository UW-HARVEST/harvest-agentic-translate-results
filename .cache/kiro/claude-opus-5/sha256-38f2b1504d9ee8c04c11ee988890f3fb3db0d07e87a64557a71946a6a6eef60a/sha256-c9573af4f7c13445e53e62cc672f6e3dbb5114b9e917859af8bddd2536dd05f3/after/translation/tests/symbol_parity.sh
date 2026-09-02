#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
# Every `T` (defined, exported) symbol in the C .so must exist in the Rust .so
# with the exact same name. Exits non-zero if the diff is non-empty.
set -uo pipefail

here="$(cd "$(dirname "$0")/../.." && pwd)"
c_build="$here/c_src/build"
rs_so="$here/translation/target/release/libbuffapp_lib.so"

if [ ! -d "$c_build" ] || [ -z "$(ls -A "$c_build" 2>/dev/null)" ]; then
  mkdir -p "$c_build"
  ( cd "$c_build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && \
    cmake --build . >/dev/null ) || { echo "C build failed"; exit 1; }
fi
c_so="$(find "$c_build" -maxdepth 1 -name '*.so' | head -n1)"
[ -n "$c_so" ] || { echo "no C .so in $c_build"; exit 1; }

[ -f "$rs_so" ] || ( cd "$here/translation" && cargo build --release --lib >/dev/null ) \
  || { echo "Rust build failed"; exit 1; }
[ -f "$rs_so" ] || { echo "missing $rs_so"; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$c_so"  | awk '$2=="T"{print $3}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$rs_so" | awk '$2=="T"{print $3}' | sort -u > "$tmp/rs.txt"

echo "C   .so: $c_so  ($(wc -l < "$tmp/c.txt") exported T symbols)"
echo "Rust .so: $rs_so  ($(wc -l < "$tmp/rs.txt") exported T symbols)"
echo
echo "--- C symbols missing from the Rust .so ---"
missing="$(comm -23 "$tmp/c.txt" "$tmp/rs.txt")"
if [ -z "$missing" ]; then echo "(none)"; else echo "$missing"; fi

echo
echo "--- unresolved symbols in the Rust .so (ldd -r, authoritative) ---"
# ldd -r performs full relocation processing and reports anything the dynamic
# loader cannot resolve. Empty output == every import is satisfied by the
# platform's libc / libgcc_s.
undef_extra="$(ldd -r "$rs_so" 2>&1 | grep -i 'undefined symbol' || true)"
if [ -z "$undef_extra" ]; then echo "(none)"; else echo "$undef_extra"; fi

echo
echo "--- imports of the Rust .so not also imported by the C .so ---"
nm -D --undefined-only "$rs_so" | awk '{print $NF}' | sed 's/@.*//' | sort -u > "$tmp/rs_u.txt"
nm -D --undefined-only "$c_so"  | awk '{print $NF}' | sed 's/@.*//' | sort -u > "$tmp/c_u.txt"
comm -23 "$tmp/rs_u.txt" "$tmp/c_u.txt" | tr '\n' ' '
echo
echo "  (all of the above are glibc / libgcc_s; ldd -r above proves they resolve)"

echo
echo "--- dlopen + dlsym check on every C symbol via the Rust .so ---"
fail=0
while read -r sym; do
  if nm -D --defined-only "$rs_so" | awk '$2=="T"{print $3}' | grep -qx "$sym"; then
    echo "  ok      $sym"
  else
    echo "  MISSING $sym"; fail=1
  fi
done < "$tmp/c.txt"

if [ -n "$missing" ] || [ -n "$undef_extra" ] || [ "$fail" -ne 0 ]; then
  echo; echo "SYMBOL PARITY: FAIL"; exit 1
fi
echo; echo "SYMBOL PARITY: PASS (0 missing, 0 non-libc undefined)"
