#!/bin/sh
# Compare the exported dynamic symbols of the C reference build and the Rust build.
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="${TMPDIR:-/tmp}/pngverify"
mkdir -p "$TMP"
if [ ! -f "$TMP/c/libpng.so" ]; then
   mkdir -p "$TMP/c"
   (cd "$TMP/c" && cmake "$ROOT/c_src" -DCMAKE_BUILD_TYPE=Release >/dev/null && make -j8 >/dev/null)
fi
(cd "$ROOT" && cargo build --release --offline >/dev/null)

nm -D --defined-only "$TMP/c/libpng.so" | awk '$2!="t"&&$2!="d"&&$2!="b"{print $2, $3}' | grep -E '^[TRDB] ' | sort -k2 > "$TMP/sym_c.txt"
nm -D --defined-only "$ROOT/target/release/libpng.so" | awk '{print $2, $3}' | grep -E '^[TRDBVW] ' | grep -E 'png' | sort -k2 > "$TMP/sym_rust_all.txt"

awk '{print $2}' "$TMP/sym_c.txt" | sort > "$TMP/names_c.txt"
awk '{print $2}' "$TMP/sym_rust_all.txt" | sort > "$TMP/names_rust.txt"

missing=$(comm -23 "$TMP/names_c.txt" "$TMP/names_rust.txt")
extra=$(comm -13 "$TMP/names_c.txt" "$TMP/names_rust.txt")

echo "C exports:    $(wc -l < "$TMP/names_c.txt")"
echo "Rust exports: $(wc -l < "$TMP/names_rust.txt")"
if [ -n "$missing" ]; then
   echo "MISSING FROM RUST:"; echo "$missing"
else
   echo "No missing symbols."
fi
if [ -n "$extra" ]; then
   echo "EXTRA IN RUST (informational):"; echo "$extra"
fi
[ -z "$missing" ]
