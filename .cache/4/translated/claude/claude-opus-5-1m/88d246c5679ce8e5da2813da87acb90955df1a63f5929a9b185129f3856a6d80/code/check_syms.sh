#!/bin/bash
# Compare exported symbols of the Rust .so against the reference C .so.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
CSO="${CSO:-$TMPDIR/cbuild/libsodium.so}"
RSO="$ROOT/target/release/libsodium.so"
nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u > /tmp/claude/c_syms.txt 2>/dev/null || nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u > "$ROOT/.c_syms.txt"
nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u > "$ROOT/.c_syms.txt"
nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u > "$ROOT/.r_syms.txt"
echo "C symbols:    $(wc -l < "$ROOT/.c_syms.txt")"
echo "Rust symbols: $(wc -l < "$ROOT/.r_syms.txt")"
echo "--- MISSING from Rust ---"
comm -23 "$ROOT/.c_syms.txt" "$ROOT/.r_syms.txt"
echo "--- EXTRA in Rust (informational) ---"
comm -13 "$ROOT/.c_syms.txt" "$ROOT/.r_syms.txt" | grep -v '^_\?\(rust_\|__rust\|_ZN\)' | head -40
