#!/usr/bin/env bash
# Runs the differential driver against the C and the Rust shared objects and
# diffs the transcripts.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

gcc -O1 -w -o difftest/driver difftest/driver.c -ldl || exit 1

if [ ! -f cbuild/libzstd.so ]; then echo "missing cbuild/libzstd.so" >&2; exit 1; fi
if [ ! -f target/release/libzstd.so ]; then echo "missing target/release/libzstd.so" >&2; exit 1; fi

./difftest/driver "$ROOT/cbuild/libzstd.so"        > difftest/c_out.txt    2> difftest/c_err.txt
CST=$?
./difftest/driver "$ROOT/target/release/libzstd.so" > difftest/rust_out.txt 2> difftest/rust_err.txt
RST=$?

echo "C driver exit=$CST  ($(wc -l < difftest/c_out.txt) lines)"
echo "Rust driver exit=$RST  ($(wc -l < difftest/rust_out.txt) lines)"

if diff -q difftest/c_out.txt difftest/rust_out.txt > /dev/null; then
    echo "=== TRANSCRIPTS IDENTICAL ($(wc -l < difftest/c_out.txt) lines) ==="
    exit 0
else
    echo "=== TRANSCRIPTS DIFFER ==="
    diff difftest/c_out.txt difftest/rust_out.txt > difftest/diff.txt
    echo "total differing lines: $(grep -c '^[<>]' difftest/diff.txt)"
    echo "--- first 60 diff lines ---"
    head -60 difftest/diff.txt
    exit 1
fi
