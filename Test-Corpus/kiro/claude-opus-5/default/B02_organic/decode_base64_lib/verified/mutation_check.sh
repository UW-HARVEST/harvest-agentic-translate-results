#!/usr/bin/env bash
# Mutation sweep: proves the differential suite actually has detection power.
# Each mutation injects a plausible translation bug into src/lib.rs; the suite
# MUST fail for every one. A mutation that survives means a blind spot.
set -uo pipefail
cd "$(dirname "$0")"
cp src/lib.rs /tmp/lib.rs.orig
trap 'cp /tmp/lib.rs.orig src/lib.rs' EXIT

declare -a NAME SED
add() { NAME+=("$1"); SED+=("$2"); }

add "decode fallthrough 63->62"          's|^    63$|    62|'
add "decode upper offset off-by-one"     's|c\.wrapping_sub(b.A. as c_char)) as c_uchar|c.wrapping_sub(b'"'"'A'"'"' as c_char).wrapping_add(1)) as c_uchar|'
add "decode digit base 52->53"           's|wrapping_add(52)|wrapping_add(53)|'
add "plus returns 63 not 62"             's|^        return 62;$|        return 63;|'
add "is_base64 drops the = case"         's|(c == b.=. as c_char)|(false)|'
add "is_base64 drops the / case"         's|(c == b./. as c_char)|(false)|'
add "calloc slack 13->12 (short buf)"    's|wrapping_add(13)|wrapping_add(12)|'
add "c3 padding gate inverted"           "s|if c3 != b'=' as c_char|if c3 == b'=' as c_char|"
add "c4 padding gate inverted"           "s|if c4 != b'=' as c_char|if c4 == b'=' as c_char|"
add "group stride 4->3"                  's|^            k += 4;$|            k += 3;|'
add "b2 mask 0xf -> 0x1f"                's|(b2 \& 0xf)|(b2 \& 0x1f)|'
add "b3 mask 0x3 -> 0x7"                 's|(b3 \& 0x3)|(b3 \& 0x7)|'
add "b1 shift 2 -> 3"                    's|(b1 << 2)|(b1 << 3)|'
add "b2 shift 4 -> 5"                    's|(b2 >> 4)|(b2 >> 5)|'
add "empty-string guard dropped"         's|if !src\.is_null() \&\& \*src != 0 {|if !src.is_null() {|'
add "c2 default A -> B"                  's|let mut c2: c_char = b.A. as c_char;|let mut c2: c_char = b"B"[0] as c_char;|'
add "k+2 bound off-by-one"               's|if k + 2 < l {|if k + 2 <= l {|'

caught=0; survived=0
for i in "${!NAME[@]}"; do
  cp /tmp/lib.rs.orig src/lib.rs
  if ! sed -i "${SED[$i]}" src/lib.rs 2>/dev/null; then
    echo "SKIP (sed error)   : ${NAME[$i]}"; continue
  fi
  if cmp -s /tmp/lib.rs.orig src/lib.rs; then
    echo "SKIP (no textual change; pattern did not match): ${NAME[$i]}"; continue
  fi
  if ! timeout 300 cargo build --release >/dev/null 2>&1; then
    echo "CAUGHT (build err) : ${NAME[$i]}"; caught=$((caught+1)); continue
  fi
  out=$(timeout 300 cargo test --release --test differential 2>&1 | grep -E '^test result' | tail -1)
  if echo "$out" | grep -q 'FAILED\|0 passed'; then
    n=$(echo "$out" | sed -E 's/.* ([0-9]+) failed.*/\1/')
    echo "CAUGHT ($n failing tests) : ${NAME[$i]}"; caught=$((caught+1))
  else
    echo "*** SURVIVED ***   : ${NAME[$i]}   [$out]"; survived=$((survived+1))
  fi
done
cp /tmp/lib.rs.orig src/lib.rs
echo
echo "caught=$caught survived=$survived"
[ "$survived" -eq 0 ] || exit 1
