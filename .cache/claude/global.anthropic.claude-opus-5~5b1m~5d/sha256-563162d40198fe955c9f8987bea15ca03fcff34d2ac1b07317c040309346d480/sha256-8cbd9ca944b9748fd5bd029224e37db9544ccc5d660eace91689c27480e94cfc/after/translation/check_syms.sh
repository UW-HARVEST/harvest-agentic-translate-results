#!/bin/bash
# Compare the exported symbols of the reference C libpng.so with the symbols
# the Rust sources declare via #[unsafe(no_mangle)].
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
C_SO="$ROOT/c_src/build/libpng.so"
OUT="${TMPDIR:-/tmp}/pngsym"
mkdir -p "$OUT"

nm -D --defined-only "$C_SO" | awk '$2!="U"{print $3}' | sort -u > "$OUT/c.txt"

# Rust: names on the line following a #[unsafe(no_mangle)] attribute
grep -A2 -h 'no_mangle' "$ROOT"/translation/src/*.rs \
  | grep -oP '(?<=fn )[A-Za-z_][A-Za-z0-9_]*|(?<=static )[A-Za-z_][A-Za-z0-9_]*' \
  | sort -u > "$OUT/rs_src.txt"

RS_SO="$ROOT/translation/target/release/libpng.so"
if [ -f "$RS_SO" ]; then
  nm -D --defined-only "$RS_SO" | awk '$2!="U"{print $3}' | sort -u > "$OUT/rs.txt"
else
  : > "$OUT/rs.txt"
fi

echo "C exports:        $(wc -l < "$OUT/c.txt")"
echo "Rust no_mangle:   $(wc -l < "$OUT/rs_src.txt")"
echo "Rust .so exports: $(wc -l < "$OUT/rs.txt")"
echo
echo "=== in C but NOT declared no_mangle in Rust sources ==="
comm -23 "$OUT/c.txt" "$OUT/rs_src.txt"
echo
echo "=== declared no_mangle in Rust but NOT in C (should be empty) ==="
comm -13 "$OUT/c.txt" "$OUT/rs_src.txt"
if [ -s "$OUT/rs.txt" ]; then
  echo
  echo "=== in C .so but MISSING from Rust .so ==="
  comm -23 "$OUT/c.txt" "$OUT/rs.txt"
fi
