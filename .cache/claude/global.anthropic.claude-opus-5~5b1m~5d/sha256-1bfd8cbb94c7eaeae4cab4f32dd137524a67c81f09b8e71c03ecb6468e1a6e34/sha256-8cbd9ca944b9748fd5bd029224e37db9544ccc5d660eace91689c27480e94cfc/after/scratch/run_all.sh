#!/bin/bash
# Build C + Rust, then run every differential test file and summarise.
set -u
cd "$(dirname "$0")/.." || exit 1
echo "== building C =="
(cd c_src/build && cmake --build . -j 8 >/dev/null) || exit 1
cd translation || exit 1
echo "== building Rust cdylib (release) =="
cargo build --offline --release 2>&1 | tail -2
echo "== symbol parity =="
nm -D --defined-only ../c_src/build/libzstd.so | awk '{print $3}' | sort -u > ../scratch/c_syms.txt
nm -D --defined-only target/release/libzstd.so | awk '{print $3}' | sort -u > ../scratch/r_syms.txt
echo "C: $(wc -l < ../scratch/c_syms.txt)  Rust: $(wc -l < ../scratch/r_syms.txt)  missing: $(comm -23 ../scratch/c_syms.txt ../scratch/r_syms.txt | wc -l)  extra: $(comm -13 ../scratch/c_syms.txt ../scratch/r_syms.txt | wc -l)"
echo "== tests =="
fail=0
total=0
for f in tests/*.rs; do
  n=$(basename "$f" .rs)
  [ "$n" = "common" ] && continue
  line=$(timeout 1200 cargo test --offline --release --test "$n" -- --test-threads=1 2>&1 | grep -E "^test result" | head -1)
  printf "%-24s %s\n" "$n" "${line:-FAILED TO RUN}"
  case "$line" in
    *"0 failed"*) c=$(echo "$line" | sed -E 's/.*ok\. ([0-9]+) passed.*/\1/'); total=$((total+c));;
    *) fail=$((fail+1));;
  esac
done
echo "TOTAL passing tests: $total ; files with failures: $fail"
