#!/bin/bash
# Diff the exported symbol sets of the C and Rust shared objects.
W=$HARVEST_WORKDIR
nm -D --defined-only "$W/verify/cbuild/libzstd.so" | awk '{print $3}' | sort -u > "$W/verify/c.txt"
nm -D --defined-only "$W/translation/target/release/libzstd.so" 2>/dev/null | awk '{print $3}' | sort -u > "$W/verify/rust.txt"
echo "C exports:    $(wc -l < "$W/verify/c.txt")"
echo "Rust exports: $(wc -l < "$W/verify/rust.txt")"
echo "--- MISSING from Rust ($(comm -23 "$W/verify/c.txt" "$W/verify/rust.txt" | wc -l)) ---"
comm -23 "$W/verify/c.txt" "$W/verify/rust.txt"
echo "--- EXTRA in Rust ($(comm -13 "$W/verify/c.txt" "$W/verify/rust.txt" | wc -l)) ---"
comm -13 "$W/verify/c.txt" "$W/verify/rust.txt"
