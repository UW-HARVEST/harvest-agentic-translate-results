#!/usr/bin/env bash
# Compare the dynamic symbols defined by the C .so against a Rust .so.
#
# Usage: compare_symbols.sh <path-to-rust.so> [path-to-c.so]
#
# Every symbol the C library defines must also be defined by the Rust library
# under the identical name. Symbols contributed by the C runtime / linker rather
# than by driver.c (_init, _fini, __bss_start, _edata, _end, __gmon_start__,
# _ITM_*, __cxa_*, __gnu_*) are excluded.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

rust_so="${1:-$root/translation/target/release/libdriver.so}"
c_so="${2:-$root/c_src/build/libdriver.so}"

for f in "$c_so" "$rust_so"; do
  if [[ ! -f "$f" ]]; then
    echo "missing shared object: $f" >&2
    exit 2
  fi
done

extract() {
  nm -D --defined-only "$1" \
    | awk '$2 ~ /^[TtDdBbRrWwVv]$/ { print $3 }' \
    | grep -vE '^(_init|_fini|__bss_start|_edata|_end|__gmon_start__)$' \
    | grep -vE '^(_ITM_|__cxa_|__gnu_)' \
    | sort -u
}

c_syms=$(extract "$c_so")
rust_syms=$(extract "$rust_so")

missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))

echo "C symbols   ($c_so):"
echo "$c_syms" | sed 's/^/    /'
echo "Rust symbols ($rust_so):"
echo "$rust_syms" | sed 's/^/    /'

if [[ -n "$missing" ]]; then
  echo "MISSING from the Rust .so:"
  echo "$missing" | sed 's/^/    /'
  exit 1
fi

echo "OK: the Rust .so defines every symbol the C .so defines"
