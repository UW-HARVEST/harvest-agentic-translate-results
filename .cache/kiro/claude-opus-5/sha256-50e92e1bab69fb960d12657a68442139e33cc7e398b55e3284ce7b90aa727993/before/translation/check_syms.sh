#!/bin/bash
# Compare the exported symbol set of the C build with the Rust cdylib.
set -u
C_SO=/tmp/pcre2cbuild/libpcre2.so
R_SO="$(dirname "$0")/target/release/libpcre2.so"

nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDRBW]$/ {print $3}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TDRBW]$/ {print $3}' | sort -u > /tmp/r_syms.txt

echo "C exports: $(wc -l < /tmp/c_syms.txt)   Rust exports: $(wc -l < /tmp/r_syms.txt)"
echo
echo "=== MISSING from Rust ($(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt | wc -l)) ==="
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt
