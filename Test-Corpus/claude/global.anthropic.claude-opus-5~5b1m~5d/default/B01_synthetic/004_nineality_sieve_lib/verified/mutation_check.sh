#!/usr/bin/env bash
# Harness self-check: each mutation below is a plausible mistranslation of
# c_src/src/sieve.c. The differential suite MUST fail for every one of them.
# (A suite that passes a mutated translation is vacuous.)
set -u
cd "$(dirname "$0")"
# Pristine snapshot of the *current* sources; restored on every exit path.
ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
trap 'cp "$ORIG" src/lib.rs; rm -f "$ORIG"' EXIT

declare -a NAME FROM TO
add() { NAME+=("$1"); FROM+=("$2"); TO+=("$3"); }

add "wrong terminator digit (9 -> 8)"        'val % 10 == 9'          'val % 10 == 8'
add "euclidean instead of truncated modulo" 'val % 10 == 9'          'val.rem_euclid(10) == 9'
add "also break on negative -9"             'val % 10 == 9'          'val % 10 == 9 || val % 10 == -9'
add "off-by-one increment (+2)"             'val.wrapping_add(1)'    'val.wrapping_add(2)'
add "saturating instead of wrapping add"    'val.wrapping_add(1)'    'val.saturating_add(1)'
add "check before print (do/while -> while)" 'printf(b"%d\n\0"'      'if val % 10 == 9 { return } printf(b"%d\n\0"'
add "wrong newline (CRLF)"                  '%d\n\0'                 '%d\r\n\0'
add "unsigned formatting (%u)"              '%d\n\0'                 '%u\n\0'
add "long formatting (%ld)"                 '%d\n\0'                 '%ld\n\0'

fail=0
for i in "${!NAME[@]}"; do
  cp "$ORIG" src/lib.rs
  python3 - "${FROM[$i]}" "${TO[$i]}" <<'PY'
import sys
frm, to = sys.argv[1], sys.argv[2]
p = 'src/lib.rs'
s = open(p).read()
body = s.split('pub extern "C" fn sieve', 1)
assert frm in body[1], f'mutation pattern not found in fn body: {frm!r}'
body[1] = body[1].replace(frm, to, 1)
open(p, 'w').write('pub extern "C" fn sieve'.join(body))
PY
  if [ $? -ne 0 ]; then echo "SKIP  ${NAME[$i]} (pattern not found)"; fail=1; continue; fi

  out=$(timeout 600 cargo test --offline 2>&1)
  if echo "$out" | grep -qE '^test result: FAILED|error\[|error:'; then
    n=$(echo "$out" | grep -cE '\.\.\. FAILED')
    echo "CAUGHT  ${NAME[$i]}  ($n failing tests)"
  else
    echo "MISSED  ${NAME[$i]}  <-- differential suite is blind to this!"
    fail=1
  fi
done

cp "$ORIG" src/lib.rs
echo
if [ "$fail" -eq 0 ]; then echo "all mutations caught"; else echo "SOME MUTATIONS MISSED"; fi
exit $fail
