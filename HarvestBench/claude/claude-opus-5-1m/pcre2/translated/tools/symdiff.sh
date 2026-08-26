#!/bin/bash
# Compares the exported symbols of the C and Rust libpcre2.so
C_SO="$1"
R_SO="$2"
T=$(mktemp -d)
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > "$T/c"
nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > "$T/r"
echo "C symbols:    $(wc -l < "$T/c")"
echo "Rust symbols: $(wc -l < "$T/r")"
echo "--- missing from Rust:"
comm -23 "$T/c" "$T/r"
echo "--- extra in Rust (informational):"
comm -13 "$T/c" "$T/r"
rm -rf "$T"
