#!/usr/bin/env bash
# Phase D: symbol parity between the C .so and the Rust .so.
#
# Every symbol the C .so exports must be exported by the Rust .so under the
# exact same name. Also reports any undefined symbol in the Rust .so that is
# not satisfied by libc / libgcc.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
C_SO="${C_DRIVER_SO:-$here/../c_src/build/libdriver.so}"
R_SO="${RUST_DRIVER_SO:-$here/target/release/libdriver.so}"

for so in "$C_SO" "$R_SO"; do
    [[ -f "$so" ]] || { echo "MISSING shared object: $so" >&2; exit 1; }
done

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Exported (defined) dynamic symbols, names only.
nm -D --defined-only "$C_SO" | awk '{print $3}' | grep -v '^$' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$R_SO" | awk '{print $3}' | grep -v '^$' | sort -u > "$tmp/r.txt"

echo "=== C exported symbols ($(wc -l < "$tmp/c.txt")) ==="
cat "$tmp/c.txt"

echo
echo "=== Symbols in C but MISSING from Rust ==="
comm -23 "$tmp/c.txt" "$tmp/r.txt" > "$tmp/missing.txt"
if [[ -s "$tmp/missing.txt" ]]; then
    cat "$tmp/missing.txt"
else
    echo "(none)"
fi

echo
echo "=== Rust undefined symbols NOT resolvable via libc/libgcc ==="
# Anything the loader must find elsewhere. ldd -r reports genuinely unresolved
# symbols; that is the authoritative check, better than name heuristics.
if ldd -r "$R_SO" 2>&1 | grep -i 'undefined symbol' > "$tmp/unres.txt"; then
    cat "$tmp/unres.txt"
else
    echo "(none — ldd -r reports no unresolved symbols)"
fi

echo
status=0
if [[ -s "$tmp/missing.txt" ]]; then
    echo "SYMBOL PARITY: FAIL ($(wc -l < "$tmp/missing.txt") missing)"
    status=1
else
    echo "SYMBOL PARITY: PASS (symbol diff is empty)"
fi
if [[ -s "$tmp/unres.txt" ]]; then
    echo "LINKABILITY: FAIL (unresolved symbols in Rust .so)"
    status=1
else
    echo "LINKABILITY: PASS (0 unresolved non-libc symbols)"
fi
exit $status
