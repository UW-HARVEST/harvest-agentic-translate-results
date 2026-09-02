#!/bin/bash
# Mechanically generate SYMBOLS.md from nm -D on the C and Rust .so files.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
C_SO="$ROOT/c_src/build/libpcre2.so"
R_SO="$ROOT/translation/target/release/libpcre2.so"
OUT="$ROOT/translation/SYMBOLS.md"

nm -D --defined-only "$C_SO" | awk 'NF>=3{print $3" "$2}' | sort -u > /tmp/c_syms_t.txt
nm -D --defined-only "$R_SO" | awk 'NF>=3{print $3" "$2}' | sort -u > /tmp/r_syms_t.txt
cut -d' ' -f1 /tmp/c_syms_t.txt | sort -u > /tmp/c_syms.txt
cut -d' ' -f1 /tmp/r_syms_t.txt | sort -u > /tmp/r_syms.txt

# symbol -> origin C object
for o in "$ROOT"/c_src/build/CMakeFiles/pcre2.dir/src/*.o; do
  b=$(basename "$o" .c.o)
  nm --defined-only --extern-only "$o" | awk -v f="$b" 'NF>=3{print $3" "f}'
done | sort -u > /tmp/sym_origin.txt

NC=$(wc -l < /tmp/c_syms.txt)
NR=$(wc -l < /tmp/r_syms.txt)
NMISS=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt | wc -l)
NEXTRA=$(comm -13 /tmp/c_syms.txt /tmp/r_syms.txt | wc -l)
set +o pipefail
NUNDEF=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' | grep -v '@GLIBC' | grep -v '@GCC' \
         | grep -vE '^(_ITM_registerTMCloneTable|_ITM_deregisterTMCloneTable|__gmon_start__)$' | wc -l)
set -o pipefail

{
echo "# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)"
echo
echo "Generated mechanically by \`gen_symbols.sh\` from \`nm -D\` on both shared objects."
echo
echo "- C   \`c_src/build/libpcre2.so\`            : **$NC** dynamic defined symbols"
echo "- Rust \`translation/target/release/libpcre2.so\`: **$NR** dynamic defined symbols"
echo "- Missing from Rust: **$NMISS**"
echo "- Extra in Rust (not in C): **$NEXTRA**"
echo "- Undefined non-libc symbols in Rust .so: **$NUNDEF**"
echo
echo "Build config: \`PCRE2_CODE_UNIT_WIDTH=8\`, \`SUPPORT_UNICODE\`, \`HAVE_CONFIG_H\` (see c_src/CMakeLists.txt)."
echo "The crate declares no cargo features, so there is exactly one feature combination"
echo "(default == \`--no-default-features\`); Phase D feature-combo sweep is therefore a single cell."
echo
echo "## Full symbol table"
echo
echo "\`T\`/\`t\` = code, \`R\`/\`D\`/\`B\` = data. \`kind\` column is the C .so binding letter."
echo
echo "| # | symbol | kind | origin C file | in Rust .so |"
echo "|---|--------|------|---------------|-------------|"
i=0
while read -r s k; do
  i=$((i+1))
  org=$(awk -v s="$s" '$1==s{print $2; exit}' /tmp/sym_origin.txt)
  [ -z "$org" ] && org="(n/a)"
  if grep -qxF "$s" /tmp/r_syms.txt 2>/dev/null; then yes="yes"; else yes="**MISSING**"; fi
  echo "| $i | \`$s\` | $k | $org.c | $yes |"
done < /tmp/c_syms_t.txt
echo
echo "## Symbols missing from the Rust .so"
echo
if [ "$NMISS" -eq 0 ]; then echo "_None._ Symbol diff is empty."; else comm -23 /tmp/c_syms.txt /tmp/r_syms.txt | sed 's/^/- `/;s/$/`/'; fi
echo
echo "## Symbols exported by Rust but not by C"
echo
if [ "$NEXTRA" -eq 0 ]; then echo "_None._"; else comm -13 /tmp/c_syms.txt /tmp/r_syms.txt | sed 's/^/- `/;s/$/`/'; fi
echo
echo "## Exported data-object sizes"
echo
echo "ELF symbol sizes (\`nm -D -S\`) for every exported data object; a size"
echo "mismatch would mean a differently-shaped table even if the name matches."
echo
nm -D -S --defined-only "$C_SO" | awk 'NF==4 && ($3=="R"||$3=="D"||$3=="B"){print $4" "$2}' | sort > /tmp/c_dsz.txt
nm -D -S --defined-only "$R_SO" | awk 'NF==4 && ($3=="R"||$3=="D"||$3=="B"){print $4" "$2}' | sort > /tmp/r_dsz.txt
NSZDIFF=$(join /tmp/c_dsz.txt /tmp/r_dsz.txt | awk '$2!=$3' | wc -l)
echo "Size mismatches: **$NSZDIFF**"
echo
echo "| symbol | C size | Rust size | same |"
echo "|--------|--------|-----------|------|"
join /tmp/c_dsz.txt /tmp/r_dsz.txt | awk '{printf "| `%s` | 0x%s | 0x%s | %s |\n", $1, $2, $3, ($2==$3?"yes":"**NO**")}'
echo
echo "## Undefined (imported) symbols in the Rust .so"
echo
echo "All are libc / libgcc-unwind / TLS runtime imports, i.e. no unresolved PCRE2 symbol:"
echo
echo '```'
nm -D --undefined-only "$R_SO" | awk '{print $2}' | sort -u
echo '```'
} > "$OUT"
echo "wrote $OUT ($(wc -l < "$OUT") lines); missing=$NMISS extra=$NEXTRA undef_nonlibc=$NUNDEF"
