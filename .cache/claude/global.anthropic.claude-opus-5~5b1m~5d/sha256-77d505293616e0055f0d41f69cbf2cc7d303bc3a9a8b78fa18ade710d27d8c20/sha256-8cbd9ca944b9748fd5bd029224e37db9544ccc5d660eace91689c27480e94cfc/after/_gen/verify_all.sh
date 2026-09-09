#!/bin/bash
# Full verification: build, symbol diff, data equality, curated harness, fuzz harness.
set -u
R=$HARVEST_WORKDIR
RS=$R/translation/target/release/libpcre2.so
C=$R/_cbuild/libpcre2.so

echo "=== build ==="
(cd $R/translation && cargo build --release 2>&1 | tail -3)
[ -f $RS ] || { echo "FAIL: no Rust .so"; exit 1; }

echo "=== symbol diff (C vs Rust) ==="
nm -D --defined-only $C  | awk '{print $3}' | sort > $R/_c_names.txt
nm -D --defined-only $RS | awk '{print $3}' | sort > $R/_rust_syms.txt
echo "C symbols: $(wc -l < $R/_c_names.txt)  Rust symbols: $(wc -l < $R/_rust_syms.txt)"
echo "-- missing in Rust:"; comm -23 $R/_c_names.txt $R/_rust_syms.txt
echo "-- extra in Rust:";   comm -13 $R/_c_names.txt $R/_rust_syms.txt

echo "=== data tables and leaf functions ==="
cd $R/_gen && ./dataeq $C $RS | tail -4

echo "=== curated API harness ==="
cd $R/_gen
gcc -O1 -o harness_rust harness.c -I$R/c_src/include -L$(dirname $RS) -lpcre2 -Wl,-rpath,$(dirname $RS) || exit 1
[ -f $R/_out_c.txt ] || { gcc -O1 -o harness_c harness.c -I$R/c_src/include -L$R/_cbuild -lpcre2 -Wl,-rpath,$R/_cbuild && ./harness_c > $R/_out_c.txt 2>&1; }
timeout 1800 ./harness_rust > $R/_out_rust.txt 2>&1; echo "rust exit=$?"
if cmp -s $R/_out_c.txt $R/_out_rust.txt; then echo "HARNESS: IDENTICAL ($(wc -l < $R/_out_c.txt) lines)";
else echo "HARNESS: DIFFERS"; diff $R/_out_c.txt $R/_out_rust.txt | head -30; fi

echo "=== randomised harness ==="
N=${1:-800}
gcc -O1 -o fuzz_rust fuzz.c -I$R/c_src/include -L$(dirname $RS) -lpcre2 -Wl,-rpath,$(dirname $RS) || exit 1
gcc -O1 -o fuzz_c fuzz.c -I$R/c_src/include -L$R/_cbuild -lpcre2 -Wl,-rpath,$R/_cbuild || exit 1
timeout 1800 ./fuzz_c $N > $R/_fuzz_c.txt 2>&1
timeout 1800 ./fuzz_rust $N > $R/_fuzz_rust.txt 2>&1; echo "rust fuzz exit=$?"
if cmp -s $R/_fuzz_c.txt $R/_fuzz_rust.txt; then echo "FUZZ: IDENTICAL ($(wc -l < $R/_fuzz_c.txt) lines)";
else echo "FUZZ: DIFFERS"; diff $R/_fuzz_c.txt $R/_fuzz_rust.txt | head -30; fi
