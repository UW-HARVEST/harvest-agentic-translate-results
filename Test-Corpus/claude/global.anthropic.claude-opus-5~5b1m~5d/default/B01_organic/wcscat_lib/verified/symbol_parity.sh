#!/usr/bin/env bash
# Phase D: the symbol diff between the C .so and the Rust .so must be EMPTY.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=$(ls ../c_src/build/*.so | head -1)
R_SO=target/debug/libwcscat_lib.so
[ -f "$R_SO" ] || R_SO=target/release/libwcscat_lib.so

echo "C   .so: $C_SO"
echo "Rust.so: $R_SO"
echo

# Defined dynamic symbols, names only.
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > .c_syms
nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > .r_syms

echo "=== C defined dynamic symbols ($(wc -l < .c_syms)) ==="
cat .c_syms
echo
echo "=== Rust defined dynamic symbols ($(wc -l < .r_syms)) ==="
cat .r_syms
echo
echo "=== MISSING from Rust (in C but not Rust) -- MUST be empty ==="
comm -23 .c_syms .r_syms | tee .missing
echo "=== EXTRA in Rust (informational) ==="
comm -13 .c_syms .r_syms

n_missing=$(wc -l < .missing)
rm -f .c_syms .r_syms .missing
echo
if [ "$n_missing" -ne 0 ]; then
  echo "RESULT: FAIL -- $n_missing C symbol(s) missing from the Rust .so"
  exit 1
fi
echo "RESULT: PASS -- 0 missing symbols"
