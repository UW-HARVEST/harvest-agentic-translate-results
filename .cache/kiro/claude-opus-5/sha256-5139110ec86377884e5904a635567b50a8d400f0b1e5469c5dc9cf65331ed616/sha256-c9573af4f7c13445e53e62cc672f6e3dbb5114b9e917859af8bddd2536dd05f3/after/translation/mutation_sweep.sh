#!/usr/bin/env bash
# Mutation sweep: the suite's value is measured by what it CATCHES, not by the
# fact that it passes. Each mutation below perturbs one behaviour of the Rust
# translation; the suite must fail for every one of them.
#
# Note the forced rebuild after every edit: cargo's mtime fingerprinting can
# consider a same-second source edit "up to date" and silently test a STALE .so.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; touch src/lib.rs; }
trap 'restore; rm -f "$ORIG"' EXIT

# Each entry: description ::: python-old ::: python-new
MUTATIONS=(
  "decode fall-through 63 -> 62:::    63\n}:::    62\n}"
  "decode upper offset 'A' -> 'B':::wrapping_sub(b'A' as c_char)) as c_uchar:::wrapping_sub(b'B' as c_char)) as c_uchar"
  "decode lower offset 26 -> 27:::wrapping_add(26)) as c_uchar:::wrapping_add(27)) as c_uchar"
  "decode digit offset 52 -> 53:::wrapping_add(52)) as c_uchar:::wrapping_add(53)) as c_uchar"
  "decode '+' 62 -> 61:::        return 62;:::        return 61;"
  "is_base64 stops accepting '=':::|| (c == b'=' as c_char):::|| false"
  "is_base64 stops accepting '/':::|| (c == b'/' as c_char):::|| false"
  "is_base64 also accepts '-':::|| (c == b'+' as c_char):::|| (c == b'+' as c_char) || (c == b'-' as c_char)"
  "c3 padding check uses '+' instead of '=':::if c3 != b'=' as c_char:::if c3 != b'+' as c_char"
  "c4 padding check uses '+' instead of '=':::if c4 != b'=' as c_char:::if c4 != b'+' as c_char"
  "byte0 shift (b1 << 2) -> (b1 << 3):::(b1 << 2) | (b2 >> 4):::(b1 << 3) | (b2 >> 4)"
  "byte0 shift (b2 >> 4) -> (b2 >> 5):::(b1 << 2) | (b2 >> 4):::(b1 << 2) | (b2 >> 5)"
  "byte1 mask 0xf -> 0x7:::((b2 & 0xf) << 4):::((b2 & 0x7) << 4)"
  "byte2 shift (b3 & 0x3) << 6 -> << 5:::((b3 & 0x3) << 6):::((b3 & 0x3) << 5)"
  "group stride k += 4 -> k += 3:::            k += 4;:::            k += 3;"
  "guard k+3 < l -> k+3 <= l:::if k + 3 < l {:::if k + 3 <= l {"
  "guard k+1 < l -> k+1 <= l:::if k + 1 < l {:::if k + 1 <= l {"
  "default c2 'A' -> 'B':::let mut c2: c_char = b'A' as c_char;:::let mut c2: c_char = b'B' as c_char;"
  "drops the empty-string rejection:::if !src.is_null() && *src != 0 {:::if !src.is_null() {"
  "drops the NULL rejection order (deref first):::if !src.is_null() && *src != 0 {:::if *src != 0 && !src.is_null() {"
  "calloc slack +13 -> +12:::l.wrapping_add(13):::l.wrapping_add(12)"
  "l = strlen+1 -> strlen+2:::(strlen(src) as c_int).wrapping_add(1):::(strlen(src) as c_int).wrapping_add(2)"
  "omits free(dest) on malloc failure:::            free(dest as *mut c_void);\n            return std::ptr::null_mut();:::            return std::ptr::null_mut();"
)

# Mutations that are SEMANTICALLY EQUIVALENT and therefore MUST NOT be caught.
# `(b3 & 0x7) << 6` and `(b3 & 0x3) << 6` are the same value in 8 bits: bit 2
# shifts to position 8 and is truncated, in the Rust `u8` expression and in the
# C `int`-promoted expression assigned to `unsigned char` alike. Listing it keeps
# the score honest — a "miss" here is correct behaviour, not a test gap.
EQUIVALENT=(
  "byte2 mask 0x3 -> 0x7 (equivalent in 8 bits):::((b3 & 0x3) << 6):::((b3 & 0x7) << 6)"
)

pass=0; caught=0; missed=0; skipped=0; equiv_ok=0; equiv_bad=0
printf '%-52s %s\n' "MUTATION" "RESULT"
printf '%.0s-' {1..72}; echo

for entry in "${MUTATIONS[@]}"; do
  desc="${entry%%:::*}"
  rest="${entry#*:::}"
  old="${rest%%:::*}"
  new="${rest#*:::}"

  restore
  if ! OLD="$old" NEW="$new" python3 -c '
import os, sys
old = os.environ["OLD"].encode().decode("unicode_escape")
new = os.environ["NEW"].encode().decode("unicode_escape")
s = open("src/lib.rs").read()
if s.count(old) < 1:
    sys.exit(3)
open("src/lib.rs","w").write(s.replace(old, new, 1))
'; then
    printf '%-52s %s\n' "$desc" "SKIP (pattern not found)"
    skipped=$((skipped+1)); continue
  fi
  touch src/lib.rs

  if ! timeout 300 cargo build --release >/dev/null 2>&1; then
    printf '%-52s %s\n' "$desc" "CAUGHT (does not compile)"
    caught=$((caught+1)); continue
  fi
  if timeout 300 cargo test --release --quiet >/dev/null 2>&1; then
    printf '%-52s %s\n' "$desc" "*** MISSED (suite still passed) ***"
    missed=$((missed+1))
  else
    printf '%-52s %s\n' "$desc" "caught"
    caught=$((caught+1))
  fi
done

printf '\n%-52s %s\n' "--- known-equivalent mutants ---" "(expected NOT caught)"
for entry in "${EQUIVALENT[@]}"; do
  desc="${entry%%:::*}"; rest="${entry#*:::}"; old="${rest%%:::*}"; new="${rest#*:::}"
  restore
  OLD="$old" NEW="$new" python3 -c '
import os, sys
old = os.environ["OLD"].encode().decode("unicode_escape")
new = os.environ["NEW"].encode().decode("unicode_escape")
s = open("src/lib.rs").read()
if s.count(old) < 1: sys.exit(3)
open("src/lib.rs","w").write(s.replace(old, new, 1))
' || { printf '%-52s %s\n' "$desc" "SKIP"; skipped=$((skipped+1)); continue; }
  touch src/lib.rs
  timeout 300 cargo build --release >/dev/null 2>&1
  if timeout 300 cargo test --release --quiet >/dev/null 2>&1; then
    printf '%-52s %s\n' "$desc" "not caught (correct: equivalent)"
    equiv_ok=$((equiv_ok+1))
  else
    printf '%-52s %s\n' "$desc" "*** caught (unexpected) ***"
    equiv_bad=$((equiv_bad+1))
  fi
done

restore
timeout 300 cargo build --release >/dev/null 2>&1
if timeout 300 cargo test --release --quiet >/dev/null 2>&1; then
  printf '\n%-52s %s\n' "unmutated original" "PASSES (as required)"
  pass=1
else
  printf '\n%-52s %s\n' "unmutated original" "*** FAILS — suite is broken ***"
fi

echo
echo "caught=$caught  missed=$missed  skipped=$skipped  equivalent_ok=$equiv_ok  original_passes=$pass"
[ "$missed" -eq 0 ] && [ "$skipped" -eq 0 ] && [ "$pass" -eq 1 ]
