#!/usr/bin/env bash
# Phase D — exported-symbol parity between the C .so and the Rust .so.
# Exits non-zero if the Rust .so is missing any symbol the C .so exports,
# or if either .so has unresolved (non-libc) undefined symbols.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root="$(dirname "$here")"
c_so="$root/c_src/build/libdriver.so"
profile="${1:-release}"
rust_so="$here/target/$profile/libdriver.so"

for f in "$c_so" "$rust_so"; do
    [[ -f "$f" ]] || { echo "MISSING: $f" >&2; exit 2; }
done

defined() { nm -D --defined-only "$1" | awk '$2 ~ /^[TWiD]$/ {print $3}' | sort -u; }

c_syms="$(mktemp)"; r_syms="$(mktemp)"
trap 'rm -f "$c_syms" "$r_syms"' EXIT
defined "$c_so"    > "$c_syms"
defined "$rust_so" > "$r_syms"

echo "=== C .so defined dynamic symbols ($(wc -l < "$c_syms")) ==="
cat "$c_syms"
echo "=== Rust .so defined dynamic symbols ($(wc -l < "$r_syms")) ==="
cat "$r_syms"

missing="$(comm -23 "$c_syms" "$r_syms")"
extra="$(comm -13 "$c_syms" "$r_syms")"

echo "=== symbols in C but NOT in Rust ==="
echo "${missing:-(none)}"
echo "=== symbols in Rust but NOT in C (informational) ==="
echo "${extra:-(none)}"

status=0
if [[ -n "$missing" ]]; then
    echo "FAIL: Rust .so is missing $(echo "$missing" | wc -l) C symbol(s)" >&2
    status=1
fi

echo "=== unresolved symbols (ldd -r) ==="
for f in "$c_so" "$rust_so"; do
    out="$(ldd -r "$f" 2>&1 | grep -i 'undefined symbol' || true)"
    if [[ -n "$out" ]]; then
        echo "FAIL: unresolved symbols in $f:" >&2
        echo "$out" >&2
        status=1
    else
        echo "ok: $f fully resolved"
    fi
done

[[ $status -eq 0 ]] && echo "SYMBOL PARITY: OK"
exit $status
