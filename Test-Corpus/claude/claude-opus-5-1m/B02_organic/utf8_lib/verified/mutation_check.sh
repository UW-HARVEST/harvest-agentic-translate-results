#!/bin/bash
# Harness sensitivity check (NOT part of the deliverable behaviour):
# inject a small bug into src/lib.rs, confirm the differential test suite
# catches it, then restore the original source.
#
# Usage: ./mutation_check.sh
set -u
cd "$(dirname "$0")"
ORIG="${TMPDIR:-/tmp}/lib.rs.mutorig"
cp src/lib.rs "$ORIG"

restore() { cp "$ORIG" src/lib.rs; }
trap restore EXIT

pass=0
fail=0

mutate() {
  local desc="$1" from="$2" to="$3"
  restore
  python3 - "$from" "$to" <<'PY'
import sys
p = 'src/lib.rs'
s = open(p).read()
frm, to = sys.argv[1], sys.argv[2]
assert frm in s, "pattern not found: %r" % frm
open(p, 'w').write(s.replace(frm, to, 1))
PY
  if [ $? -ne 0 ]; then echo "  !! could not apply mutation: $desc"; fail=$((fail+1)); return; fi

  local out rc
  out=$(timeout 600 cargo test --offline -q 2>&1; timeout 600 cargo test --offline --release -q 2>&1)
  rc=$?
  # "caught" == the suite did not come back clean, by ANY mechanism: an
  # assertion failure, or the whole test binary dying (e.g. glibc aborting in
  # free() because the mutant overflowed its heap buffer).
  if [ "$rc" -ne 0 ] || echo "$out" | grep -qE '(FAILED|test result: FAILED)'; then
    echo "  [caught]  $desc"
    echo "$out" | grep -E '^ *(failures:)?$' >/dev/null
    echo "$out" | grep -oE '^test [a-z0-9_:]+ \.\.\. FAILED' | head -3 | sed 's/^/              /'
    echo "$out" | grep -oE 'assertion .* failed[^\n]*' | head -1 | cut -c1-140 | sed 's/^/              /'
    echo "$out" | grep -oE '(free|malloc|realloc)\(\): [a-z ]+' | head -1 | sed 's/^/              glibc: /'
    echo "$out" | grep -oE 'SIGABRT|SIGSEGV' | head -1 | sed 's/^/              died with /'
    pass=$((pass+1))
  else
    echo "  [MISSED]  $desc   <-- test suite is blind to this bug"
    fail=$((fail+1))
  fi
}

echo "=== mutation sensitivity check ==="

mutate "valid_3: 0xED surrogate guard  < 0xA0  ->  <= 0xA0" \
       '(*x != 0xED || *x.add(1) < 0xA0)' \
       '(*x != 0xED || *x.add(1) <= 0xA0)'

mutate "valid_3: 0xE0 overlong guard  >= 0xA0  ->  > 0xA0" \
       '(*x != 0xE0 || *x.add(1) >= 0xA0)' \
       '(*x != 0xE0 || *x.add(1) > 0xA0)'

mutate "valid_4: max-codepoint guard  <= 0x8F  ->  <= 0x90" \
       '(*x != 0xF4 || *x.add(1) <= 0x8F)' \
       '(*x != 0xF4 || *x.add(1) <= 0x90)'

mutate "valid_4: lead range  <= 0xF4  ->  <= 0xF7" \
       '&& *x <= 0xF4' \
       '&& *x <= 0xF7'

mutate "valid_2: overlong lead guard dropped (0xC0/0xC1 accepted)" \
       '&& (*x as i8) >= (0xC2u8 as i8)' \
       '&& true'

mutate "w_utf8_filter: bool test  != 0  ->  & 1 != 0" \
       'let replacement = replacement != 0;' \
       'let replacement = (replacement & 1) != 0;'

mutate "w_utf8_filter: repl threshold  repl < 3  ->  repl < 2" \
       'if repl < 3 {' \
       'if repl < 2 {'

mutate "w_utf8_filter: REPLACEMENT_INC 4096 -> 4095" \
       'const REPLACEMENT_INC: usize = 4096;' \
       'const REPLACEMENT_INC: usize = 4095;'

mutate "w_utf8_filter: U+FFFD bytes EF BF BD -> EF BF BE" \
       '*copy.add(i) = 0xBD;' \
       '*copy.add(i) = 0xBE;'

mutate "w_utf8_filter: drop the NULL check after malloc" \
       'let mut copy = malloc(size) as *mut u8;
        if copy.is_null() {
            return ptr::null_mut();
        }' \
       'let mut copy = malloc(size) as *mut u8;
        if false {
            return ptr::null_mut();
        }'

mutate "w_utf8_filter: drop the NULL check after realloc" \
       'copy = realloc(copy as *mut c_void, size) as *mut u8;
                        if copy.is_null() {
                            return ptr::null_mut();
                        }' \
       'copy = realloc(copy as *mut c_void, size) as *mut u8;
                        if false {
                            return ptr::null_mut();
                        }'

mutate "w_utf8_drop: skip the NULL assert" \
       'assert_string_not_null(40,' \
       'return core::ptr::null(); #[allow(unreachable_code)] assert_string_not_null(40,'

mutate "w_utf8_filter: skip the NULL assert" \
       'assert_string_not_null(60,' \
       'return core::ptr::null_mut(); #[allow(unreachable_code)] assert_string_not_null(60,'

mutate "w_utf8_drop: 4-byte advance  add(4)  ->  add(3)" \
       'string = string.add(4);' \
       'string = string.add(3);'

mutate "w_utf8_filter: valid_2/valid_3 checked in the wrong order" \
       '} else if valid_3(valid) {' \
       '} else if false && valid_3(valid) {'

mutate "valid_3: 3rd-byte check evaluated before the 2nd (reads past the NUL)" \
       '&& (*x.add(1) & 0xC0) == 0x80
        && (*x.add(2) & 0xC0) == 0x80' \
       '&& (*x.add(2) & 0xC0) == 0x80
        && (*x.add(1) & 0xC0) == 0x80'

mutate "assert(): wrong __FILE__ string" \
       'concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/src/lib.c\0")' \
       '"src/lib.c\0"'

mutate "assert(): wrong __LINE__ (40 -> 41)" \
       'assert_string_not_null(40,' \
       'assert_string_not_null(41,'

mutate "assert(): wrong __PRETTY_FUNCTION__" \
       'assert_string_not_null(60, b"w_utf8_filter\0")' \
       'assert_string_not_null(60, b"w_utf8_drop\0")'

restore
echo "=== caught: $pass   missed: $fail ==="
[ "$fail" -eq 0 ]
