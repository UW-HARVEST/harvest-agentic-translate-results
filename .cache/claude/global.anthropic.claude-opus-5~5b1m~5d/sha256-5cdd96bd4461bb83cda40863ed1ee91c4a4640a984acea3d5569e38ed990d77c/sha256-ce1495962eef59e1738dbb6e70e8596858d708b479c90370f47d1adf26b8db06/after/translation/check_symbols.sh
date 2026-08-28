#!/bin/sh
# Phase A / Phase D symbol-parity check.
# Prints the set difference of exported (defined, global) symbols.
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(dirname "$here")

CSO=$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)
RSO="$here/target/release/libarr_ins_lib.so"

if [ ! -f "$CSO" ]; then echo "missing C .so; build c_src first" >&2; exit 1; fi
if [ ! -f "$RSO" ]; then echo "missing Rust .so; cargo build --release first" >&2; exit 1; fi

nm -D --defined-only "$CSO" | awk '$2=="T"||$2=="W"{print $3}' | sort -u > "${TMPDIR:-/tmp}/csyms.$$"
nm -D --defined-only "$RSO" | awk '$2=="T"||$2=="W"{print $3}' | sort -u > "${TMPDIR:-/tmp}/rsyms.$$"

echo "C exported: $(wc -l < "${TMPDIR:-/tmp}/csyms.$$")   Rust exported: $(wc -l < "${TMPDIR:-/tmp}/rsyms.$$")"
echo "--- in C but MISSING from Rust (must be empty) ---"
comm -23 "${TMPDIR:-/tmp}/csyms.$$" "${TMPDIR:-/tmp}/rsyms.$$"
missing=$(comm -23 "${TMPDIR:-/tmp}/csyms.$$" "${TMPDIR:-/tmp}/rsyms.$$" | wc -l)
echo "--- extra in Rust (informational) ---"
comm -13 "${TMPDIR:-/tmp}/csyms.$$" "${TMPDIR:-/tmp}/rsyms.$$" | head -40
echo "--- unresolved symbols ---"
ldd -r "$CSO" 2>&1 | grep -i 'undefined symbol' || echo "C: none"
ldd -r "$RSO" 2>&1 | grep -i 'undefined symbol' || echo "Rust: none"
rm -f "${TMPDIR:-/tmp}/csyms.$$" "${TMPDIR:-/tmp}/rsyms.$$"
echo "MISSING_COUNT=$missing"
[ "$missing" = "0" ]
