#!/bin/sh
# mutation_sweep.sh - proves the differential suite actually has teeth.
#
# The suite passing tells you nothing unless a WRONG translation would make it
# fail. This script injects one off-by-one / wrong-constant bug at a time into
# the Rust translation, rebuilds the .so, runs the whole suite, and requires the
# suite to FAIL. It then restores the original source.
#
# Detection metric is the suite EXIT STATUS, not a grep for "FAILED": an injected
# bug can make a test assert OR crash the harness (e.g. an out-of-range read
# that yields a garbage pointer), and both must count as detected.
#
# Exit status = number of ESCAPED mutations (0 is the desired result).
# Mutation-test the differential suite: perturb one constant/comparison in the
# Rust translation, rebuild the .so, run the whole suite, and require it to FAIL.
# Detection metric is the suite's EXIT STATUS (an injected bug can make a test
# assert, or crash the harness outright, and both must count as "caught").
cd "$(dirname "$0")/../translation"
BAK=$(mktemp -d)
mkdir -p "$BAK"
for f in dump utf value load pack_unpack strconv hashtable strbuffer; do
  cp "src/$f.rs" "$BAK/$f.rs"
done

caught=0; escaped=0
mut() {
  desc="$1"; file="src/$2.rs"; from="$3"; to="$4"
  python3 - "$file" "$from" "$to" <<'PY'
import sys
p,f,t=sys.argv[1],sys.argv[2],sys.argv[3]
s=open(p).read()
if f not in s:
    sys.exit(3)
open(p,"w").write(s.replace(f,t,1))
PY
  rc=$?
  if [ $rc -eq 3 ]; then echo "SKIP  [$desc] pattern not found"; return; fi
  if cargo build --offline --release >/dev/null 2>&1; then
    if cargo test --offline --release -- --test-threads=1 >/dev/null 2>&1; then
      echo "ESCAPED  [$desc]  <-- the suite did NOT detect this"
      escaped=$((escaped+1))
    else
      echo "caught   [$desc]"
      caught=$((caught+1))
    fi
  else
    echo "SKIP  [$desc] did not compile"
  fi
  cp "$BAK/$2.rs" "$file"
}

mut "dump: ENSURE_ASCII boundary 0x7F->0x80"  dump      "codepoint > 0x7F" "codepoint > 0x80"
mut "dump: control cutoff 0x20->0x1f"          dump      "codepoint < 0x20" "codepoint < 0x1f"
mut "utf: surrogate hi bound 0xDFFF->0xDFFE"   utf       "(0xD800..=0xDFFF).contains(&value)" "(0xD800..=0xDFFE).contains(&value)"
mut "utf: max cp 0x10FFFF->0x10FFFE"           utf       "if value > 0x10FFFF {" "if value > 0x10FFFE {"
mut "utf: encode max cp 0x10FFFF->0x10FFFE"    utf       "codepoint <= 0x10FFFF" "codepoint <= 0x10FFFE"
mut "value: array_get bound >= -> >"            value     "if index >= (*array).entries {" "if index > (*array).entries {"
mut "value: array_insert bound > -> >="          value     "if index > (*array).entries {" "if index >= (*array).entries {"
mut "load: depth limit > -> >="                   load      "if (*lex).depth > JSON_PARSER_MAX_DEPTH {" "if (*lex).depth >= JSON_PARSER_MAX_DEPTH {"
mut "hashtable: initial order 3->4"               hashtable "pub const INITIAL_HASHTABLE_ORDER: usize = 3;" "pub const INITIAL_HASHTABLE_ORDER: usize = 4;"
mut "hashtable: rehash threshold >= -> >"         hashtable "(*hashtable).size >= hashsize((*hashtable).order)" "(*hashtable).size > hashsize((*hashtable).order)"
mut "strbuffer: MIN_SIZE 16->32"                  strbuffer "const STRBUFFER_MIN_SIZE: usize = 16;" "const STRBUFFER_MIN_SIZE: usize = 32;"
mut "strbuffer: FACTOR 2->3"                      strbuffer "const STRBUFFER_FACTOR: usize = 2;" "const STRBUFFER_FACTOR: usize = 3;"
mut "strconv: exponent switch 16->15"             strconv   "if decpt <= -4 || decpt > 16 {" "if decpt <= -4 || decpt > 15 {"
mut "strconv: exponent switch -4 -> -3"           strconv   "if decpt <= -4 ||" "if decpt <= -3 ||"

echo
echo "caught=$caught escaped=$escaped"
cargo build --offline --release >/dev/null 2>&1
exit $escaped
