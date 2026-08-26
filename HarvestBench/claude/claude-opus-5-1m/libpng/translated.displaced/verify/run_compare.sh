#!/bin/sh
# Build the C reference library and the Rust translation, run both behavioural
# harnesses against each and diff the (deterministic) output byte for byte.
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="${TMPDIR:-/tmp}/pngverify"
mkdir -p "$TMP"

# --- C reference build
if [ ! -f "$TMP/c/libpng.so" ]; then
   mkdir -p "$TMP/c"
   (cd "$TMP/c" && cmake "$ROOT/c_src" -DCMAKE_BUILD_TYPE=Release >/dev/null && make -j8 >/dev/null)
fi

# --- Rust build
(cd "$ROOT" && cargo build --release --offline >/dev/null)
mkdir -p "$TMP/rust"
cp "$ROOT/target/release/libpng.so" "$TMP/rust/libpng.so"

status=0
for h in harness harness2; do
   gcc -O1 -I "$ROOT/c_src/include" -o "$TMP/$h" "$ROOT/verify/$h.c" \
       -L"$TMP/c" -lpng -lz -lm 2>/dev/null
   LD_LIBRARY_PATH="$TMP/c"    "$TMP/$h" > "$TMP/${h}_c.txt"    2>&1 || true
   LD_LIBRARY_PATH="$TMP/rust" "$TMP/$h" > "$TMP/${h}_rust.txt" 2>&1 || true
   if diff -u "$TMP/${h}_c.txt" "$TMP/${h}_rust.txt" > "$TMP/${h}_diff.txt"; then
      echo "$h: IDENTICAL ($(wc -l < "$TMP/${h}_c.txt") lines)"
   else
      echo "$h: DIFFERENCES ($(grep -c '^[-+]' "$TMP/${h}_diff.txt") diff lines)"
      head -40 "$TMP/${h}_diff.txt"
      status=1
   fi
done
exit $status
