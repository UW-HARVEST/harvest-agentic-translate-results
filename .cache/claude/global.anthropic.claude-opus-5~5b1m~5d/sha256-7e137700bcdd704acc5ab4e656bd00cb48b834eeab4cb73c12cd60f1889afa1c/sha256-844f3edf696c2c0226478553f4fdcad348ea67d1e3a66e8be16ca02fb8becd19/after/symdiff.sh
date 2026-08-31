#!/bin/bash
# Mechanical `nm -D` symbol diff, C .so vs Rust .so, for every feature combination.
set -u
root="$(cd "$(dirname "$0")" && pwd)"
cd "$root/translation" || exit 1
total_missing=0
for op in "" add sub mul; do
  for rep in "" 0 1 2 3 4 5 6 7; do
    combo="$(echo "$op $rep" | tr ' ' ',' | sed 's/^,//; s/,$//')"
    cop="${op:-add}"; crep="${rep:-5}"
    cso="$root/cbuild/libcdriver_${cop}_${crep}.so"
    [ -z "$op" ] && [ -z "$rep" ] && cso="$root/cbuild/libcdriver_default.so"
    cargo build --no-default-features --features "$combo" -q || exit 1
    missing=$(comm -23 \
      <(nm -D --defined-only --format=posix "$cso" | awk '{print $1}' | sort -u) \
      <(nm -D --defined-only --format=posix target/debug/libdriver.so | awk '{print $1}' | sort -u))
    n=$(printf '%s' "$missing" | grep -c . )
    printf 'features=[%-8s] C=%s/%-6s missing=%s %s\n' "$combo" "$cop" "$crep" "$n" "$missing"
    total_missing=$((total_missing + n))
  done
done
echo "TOTAL MISSING SYMBOLS: $total_missing"
exit $((total_missing > 0))
