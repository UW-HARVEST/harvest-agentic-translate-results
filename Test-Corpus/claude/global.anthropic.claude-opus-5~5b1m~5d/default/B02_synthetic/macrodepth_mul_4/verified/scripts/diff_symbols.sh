#!/usr/bin/env bash
# Symbol-parity check: `nm -D --defined-only` on the C .so vs the Rust .so for
# every canonical (OP, REPEAT) configuration. The diff must be empty.
set -u
cd "$(dirname "$0")/.."
ROOT="$(cd .. && pwd)"
OUT="${TMPDIR:-/tmp}/symdiff"
mkdir -p "$OUT"

fail=0
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    gcc -O2 -fPIC -shared -DOP="$op" -DREPEAT="$rep" \
        -o "$OUT/libcmd.so" "$ROOT/c_src/src/mdcore.c" \
      || { echo "cc fail $op $rep"; fail=1; continue; }
    cargo build --quiet --release --no-default-features --features "$op,$rep" \
      || { echo "rust build fail $op $rep"; fail=1; continue; }

    nm -D --defined-only "$OUT/libcmd.so" | awk '{print $3}' | sort -u > "$OUT/c.txt"
    nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u > "$OUT/r.txt"

    missing=$(comm -23 "$OUT/c.txt" "$OUT/r.txt")
    if [ -n "$missing" ]; then
      echo "MISSING in Rust .so (op=$op rep=$rep):"
      printf '%s\n' "$missing" | sed 's/^/  /'
      fail=1
    fi
  done
done
echo "symbol diff done fail=$fail"
exit $fail
