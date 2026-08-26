#!/bin/sh
# Regenerate SYMBOLS.md from nm -D on both shared objects.
set -e
W="$(cd "$(dirname "$0")" && pwd)"
C_SO="$W/c_src/build/libjansson.so"
R_SO="$W/target/release/libjansson.so"
[ -f "$R_SO" ] || R_SO="$W/target/debug/libjansson.so"
TD="${TMPDIR:-/tmp}"
nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBRWVi]$/ {print $3" "$2}' | sort -u > "$TD/c.txt"
nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TDBRWVi]$/ {print $3" "$2}' | sort -u > "$TD/r.txt"
cut -d' ' -f1 "$TD/c.txt" > "$TD/cn.txt"
cut -d' ' -f1 "$TD/r.txt" > "$TD/rn.txt"
{
  echo "# SYMBOLS.md — exported-symbol parity (C \`.so\` vs Rust \`.so\`)"
  echo
  echo "Generated mechanically by \`gen_symbols.sh\` (\`nm -D --defined-only\`)."
  echo
  echo "* C   \`.so\`: \`c_src/build/libjansson.so\`  — **$(wc -l < "$TD/cn.txt")** dynamic symbols"
  echo "* Rust \`.so\`: \`$(basename $(dirname $R_SO))/libjansson.so\` — **$(wc -l < "$TD/rn.txt")** dynamic symbols"
  echo
  echo "## Missing in Rust (\`comm -23\`)"
  echo
  if [ -s "$(comm -23 "$TD/cn.txt" "$TD/rn.txt" > "$TD/miss.txt"; echo "$TD/miss.txt")" ]; then
    echo '```'
    cat "$TD/miss.txt"
    echo '```'
  else
    echo "**NONE — 0 missing symbols.**"
  fi
  echo
  echo "## Extra jansson-namespace symbols in Rust (should be none)"
  echo
  comm -13 "$TD/cn.txt" "$TD/rn.txt" | grep -Ei '^(json|jansson|hashtable|strbuffer|utf8|dtoa|freedtoa|gethex|strtod__)' > "$TD/extra.txt" || true
  if [ -s "$TD/extra.txt" ]; then echo '```'; cat "$TD/extra.txt"; echo '```'; else echo "**NONE.**"; fi
  echo
  echo "## Undefined (imported) non-libc symbols in Rust \`.so\`"
  echo
  nm -D -u "$R_SO" | awk '{print $2}' | sed 's/@.*//' | sort -u > "$TD/rund.txt"
  # anything in the jansson namespace that is imported rather than defined is a real gap
  grep -Ei '^(json|jansson|hashtable|strbuffer|utf8|dtoa|freedtoa|gethex)' "$TD/rund.txt" > "$TD/rundj.txt" || true
  if [ -s "$TD/rundj.txt" ]; then echo '```'; cat "$TD/rundj.txt"; echo '```'; else echo "**NONE — every jansson symbol is defined, not imported.**"; fi
  echo
  echo "## Full symbol table"
  echo
  echo "| symbol | C type | Rust type | in Rust? | C source |"
  echo "|--------|--------|-----------|----------|----------|"
  while read -r name ctype; do
    rtype=$(grep -m1 "^$name " "$TD/r.txt" | cut -d' ' -f2 || true)
    if [ -n "$rtype" ]; then ok="yes"; else ok="**NO**"; rtype="-"; fi
    src=$(grep -rl -E "(^|[^A-Za-z0-9_])$name([^A-Za-z0-9_]|\$)" "$W/c_src/src" 2>/dev/null | head -1 | xargs -r basename || true)
    echo "| \`$name\` | $ctype | $rtype | $ok | $src |"
  done < "$TD/c.txt"
} > "$W/SYMBOLS.md"
echo "wrote SYMBOLS.md: $(wc -l < "$W/SYMBOLS.md") lines"
echo "missing: $(wc -l < "$TD/miss.txt")"
