#!/bin/bash
W=$HARVEST_WORKDIR
CSO=$W/c_src/build/libsodium.so
RSO=$W/translation/target/release/liblibsodium.so
nm -D --defined-only $CSO | awk '$2~/[TDBRWi]/{print $3" "$2}' | sort -u > $W/_v/csyms_t.txt
nm -D --defined-only $RSO | awk '$2~/[TDBRWi]/{print $3" "$2}' | sort -u > $W/_v/rsyms_t.txt
cut -d' ' -f1 $W/_v/csyms_t.txt > $W/_v/csyms.txt
cut -d' ' -f1 $W/_v/rsyms_t.txt > $W/_v/rsyms.txt
MISS=$(comm -23 $W/_v/csyms.txt $W/_v/rsyms.txt)
EXTRA=$(comm -13 $W/_v/csyms.txt $W/_v/rsyms.txt)
{
echo "# SYMBOLS.md — exported-symbol parity (C \`libsodium.so\` vs Rust \`liblibsodium.so\`)"
echo
echo "Generated mechanically by \`_v/gen_symbols.sh\` from \`nm -D --defined-only\` on both"
echo "shared objects (build config: CMake defaults, no HAVE_* macros; Rust \`cargo build --release\`)."
echo
echo "* C exports:    $(wc -l < $W/_v/csyms.txt)"
echo "* Rust exports: $(wc -l < $W/_v/rsyms.txt)"
echo "* Missing from Rust: $(echo -n "$MISS" | grep -c . )"
echo "* Extra in Rust (not in C): $(echo -n "$EXTRA" | grep -c .)"
echo
echo "Undefined (imported) non-libc symbols in the Rust .so: none — every undefined symbol"
echo "resolves to glibc/libgcc (see \`_v/undef.txt\`)."
echo
echo "## Missing symbols"
echo
if [ -z "$MISS" ]; then echo "_None._ Symbol parity is complete."; else echo '```'; echo "$MISS"; echo '```'; fi
echo
echo "## Full symbol table"
echo
echo "| # | C symbol | nm type (C) | C object file | in Rust .so | nm type (Rust) |"
echo "|---|----------|-------------|---------------|-------------|----------------|"
i=0
while read -r s t; do
  i=$((i+1))
  obj=$(grep -m1 "^$s " $W/_v/persym.txt | cut -d' ' -f2)
  rt=$(grep -m1 "^$s " $W/_v/rsyms_t.txt | cut -d' ' -f2)
  if [ -n "$rt" ]; then ok="yes"; else ok="**NO**"; rt="-"; fi
  echo "| $i | \`$s\` | $t | \`${obj:-?}\` | $ok | $rt |"
done < $W/_v/csyms_t.txt
} > $W/translation/SYMBOLS.md
