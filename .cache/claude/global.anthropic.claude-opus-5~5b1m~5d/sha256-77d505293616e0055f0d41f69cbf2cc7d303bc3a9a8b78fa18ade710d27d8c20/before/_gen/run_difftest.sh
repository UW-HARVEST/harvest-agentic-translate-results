#!/bin/bash
# Differential test: run the harness against the C and the Rust libpcre2.so and diff.
set -u
R=$HARVEST_WORKDIR
RUSTDIR=$R/translation/target/release
cd $R/translation && cargo build --release 2>&1 | tail -2
[ -f $RUSTDIR/libpcre2.so ] || { echo "no rust .so"; exit 1; }
cd $R/_gen
gcc -O1 -o harness_rust harness.c -I$R/c_src/include -L$RUSTDIR -lpcre2 -Wl,-rpath,$RUSTDIR || exit 1
[ -f $R/_out_c.txt ] || { gcc -O1 -o harness_c harness.c -I$R/c_src/include -L$R/_cbuild -lpcre2 -Wl,-rpath,$R/_cbuild && ./harness_c > $R/_out_c.txt 2>&1; }
timeout 1800 ./harness_rust > $R/_out_rust.txt 2>&1
echo "rust harness exit=$?  lines=$(wc -l < $R/_out_rust.txt)  (C lines=$(wc -l < $R/_out_c.txt))"
if diff -q $R/_out_c.txt $R/_out_rust.txt > /dev/null; then
  echo "IDENTICAL OUTPUT"
else
  echo "DIFFERENCES: $(diff $R/_out_c.txt $R/_out_rust.txt | grep -c '^[<>]') diff lines; first 40:"
  diff $R/_out_c.txt $R/_out_rust.txt | head -40
fi
