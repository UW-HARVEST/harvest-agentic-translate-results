#!/usr/bin/env bash
# Negative control for the differential suite.
#
# Injects deliberate bugs into src/lib.rs, one at a time, and asserts the test
# suite FAILS for each. A suite that passes a mutated translation proves nothing
# about the unmutated one. src/lib.rs is restored afterwards.
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="$CRATE_DIR/src/lib.rs"
BAK="$(mktemp)"
C_SO="$CRATE_DIR/../c_src/build/libdriver.so"
cp "$LIB" "$BAK"
restore() { cp "$BAK" "$LIB"; rm -f "$BAK"; }
trap restore EXIT

SURVIVORS=0
KILLED=0

run_mutant() {
  local name="$1"; shift
  cp "$BAK" "$LIB"
  "$@" || { printf 'SKIP  %-42s (could not apply)\n' "$name"; return; }
  if ! cmp -s "$BAK" "$LIB"; then :; else
    printf 'SKIP  %-42s (mutation was a no-op)\n' "$name"; return
  fi
  if ! timeout 600 cargo build --release --quiet >/dev/null 2>&1; then
    printf 'SKIP  %-42s (mutant does not compile)\n' "$name"; return
  fi
  if DRIVER_C_SO="$C_SO" timeout 600 cargo test --release --quiet \
       -- --test-threads=1 >/dev/null 2>&1; then
    printf 'SURVIVED  %-38s <-- the suite does NOT detect this bug\n' "$name"
    SURVIVORS=$((SURVIVORS + 1))
  else
    printf 'killed    %-38s\n' "$name"
    KILLED=$((KILLED + 1))
  fi
}

# 1. Normalise the ctype masks to 0/1 (the classic "tidy up the C" mistake).
m_normalise() {
  perl -0pi -e 's/\(\*table\.offset\(c as c_int as isize\) & mask\) as c_int/((*table.offset(c as c_int as isize) \& mask) != 0) as c_int/' "$LIB"
}

# 2. Drop the 8-bit narrowing, so the ABI's unspecified high bits leak into the
#    table index (the segfault bug: rustc marks an i8 param `signext` and then
#    trusts the caller, while GCC's code keeps only %al).
m_no_truncate() {
  perl -0pi -e 's/pub extern "C" fn driver\(c_arg: c_int\) \{\n    let c: c_char = c_arg as u8 as c_char;\n/pub extern "C" fn driver(c: c_char) {\n/' "$LIB"
}

# 9. Swap tolower and toupper.
m_swap_case() {
  perl -0pi -e 's/tolower\(c as c_int\)/__SWAP__/; s/toupper\(c as c_int\)/tolower(c as c_int)/; s/__SWAP__/toupper(c as c_int)/' "$LIB"
}

# 10. Print the case-conversion results as %d instead of %c.
m_percent_d() {
  perl -0pi -e 's/c"to lower: %c\\n"/c"to lower: %d\\n"/' "$LIB"
}

# 11. Zero-extend instead of sign-extend when narrowing the argument.
m_zero_extend() {
  perl -0pi -e 's/let c: c_char = c_arg as u8 as c_char;/let c: c_char = (c_arg \& 0x7f) as c_char;/' "$LIB"
}

# 12. Reorder: run setlocale after the classifications instead of before.
m_late_setlocale() {
  perl -0pi -e 's/setlocale\(LC_ALL, c"C"\.as_ptr\(\)\);\n/\n/; s/(printf\(c"to upper: %c\\n"\.as_ptr\(\), toupper\(c as c_int\)\);)/$1\n        setlocale(LC_ALL, c"C".as_ptr());/' "$LIB"
}

# 3. Freeze the locale: classify through a snapshot instead of the live table.
m_frozen_locale() {
  perl -0pi -e 's/let table = \*__ctype_b_loc\(\);/static mut FROZEN: *const c_ushort = core::ptr::null(); if FROZEN.is_null() { FROZEN = *__ctype_b_loc(); } let table = FROZEN;/' "$LIB"
}

# 4. Reimplement tolower with ASCII-only logic instead of calling libc (breaks
#    every non-C locale, e.g. Turkish I).
m_ascii_tolower() {
  perl -0pi -e 's/printf\(c"to lower: %c\\n"\.as_ptr\(\), tolower\(c as c_int\)\);/printf(c"to lower: %c\\n".as_ptr(), if c >= b\x27A\x27 as c_char \&\& c <= b\x27Z\x27 as c_char { c as c_int + 32 } else { c as c_int });/' "$LIB"
}

# 5. Swap two printf lines (output ordering).
m_swap_lines() {
  perl -0pi -e 's/(printf\(c"space: %d\\n"\.as_ptr\(\), isspace\(c\)\);\n\s*)(printf\(c"blank: %d\\n"\.as_ptr\(\), isblank\(c\)\);)/$2\n        printf(c"space: %d\\n".as_ptr(), isspace(c));/' "$LIB"
}

# 6. Off-by-one on a mask constant.
m_bad_mask() {
  perl -0pi -e 's/const IS_XDIGIT: c_ushort = 4096;/const IS_XDIGIT: c_ushort = 2048;/' "$LIB"
}

# 7. Drop the setlocale call.
m_no_setlocale() {
  perl -0pi -e 's/setlocale\(LC_ALL, c"C"\.as_ptr\(\)\);//' "$LIB"
}

# 8. Treat the char as unsigned, losing the negative table indices.
m_unsigned_char() {
  perl -0pi -e 's/\(\*table\.offset\(c as c_int as isize\) & mask\)/(*table.offset(c as u8 as c_int as isize) \& mask)/' "$LIB"
}

printf 'Mutation testing the differential suite\n'
printf '%s\n' "--------------------------------------------------------"
run_mutant "normalise ctype masks to 0/1"     m_normalise
run_mutant "no 8-bit narrowing of the arg"    m_no_truncate
run_mutant "frozen (non-live) ctype table"    m_frozen_locale
run_mutant "ASCII-only tolower"               m_ascii_tolower
run_mutant "swap space/blank printf order"    m_swap_lines
run_mutant "wrong _ISxdigit mask"             m_bad_mask
run_mutant "no setlocale"                     m_no_setlocale
run_mutant "unsigned char table index"        m_unsigned_char
run_mutant "swap tolower/toupper"             m_swap_case
run_mutant "%d instead of %c"                 m_percent_d
run_mutant "mask arg with 0x7f (zero-extend)" m_zero_extend
run_mutant "setlocale after, not before"      m_late_setlocale
printf '%s\n' "--------------------------------------------------------"
printf 'killed: %d   survived: %d\n' "$KILLED" "$SURVIVORS"

restore
trap - EXIT
timeout 600 cargo build --release --quiet >/dev/null 2>&1
if ((SURVIVORS > 0)); then
  printf '\n%d mutant(s) survived: the suite has blind spots.\n' "$SURVIVORS"
  exit 1
fi
printf '\nAll mutants killed.\n'
