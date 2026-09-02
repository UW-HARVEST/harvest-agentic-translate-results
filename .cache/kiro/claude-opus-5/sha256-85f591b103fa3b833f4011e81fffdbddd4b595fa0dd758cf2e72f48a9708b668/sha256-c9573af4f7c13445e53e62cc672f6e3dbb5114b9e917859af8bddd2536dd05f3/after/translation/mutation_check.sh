#!/usr/bin/env bash
# Mutation check: each mutation of the Rust translation MUST be detected by the
# differential test suite. Restores src/lib.rs afterwards and verifies the hash.
set -u
cd "$(dirname "$0")"

GOOD=/tmp/verify-backup/lib.rs.session
cp src/lib.rs "$GOOD"
trap 'cp "$GOOD" src/lib.rs; cargo build -q >/dev/null 2>&1' EXIT

apply() { python3 - "$1" "$2" <<'PY'
import sys
old, new = sys.argv[1], sys.argv[2]
s = open('src/lib.rs').read()
n = s.count(old)
assert n == 1, f"mutation target occurs {n} times (want exactly 1): {old!r}"
open('src/lib.rs','w').write(s.replace(old, new, 1))
PY
}

run_mutation() {
  local name="$1" old="$2" new="$3"
  cp "$GOOD" src/lib.rs
  if ! apply "$old" "$new" 2>/tmp/mut.err; then
    echo "SKIP     $name -- $(tail -1 /tmp/mut.err)"; return
  fi
  if ! cargo build -q >/dev/null 2>&1; then echo "SKIP     $name (does not compile)"; return; fi
  timeout 900 cargo test -q >/tmp/mut.log 2>&1
  if grep -q "FAILED" /tmp/mut.log; then
    echo "KILLED   $name  ($(grep -c '\.\.\. FAILED' /tmp/mut.log) failing tests)"
  else
    echo "SURVIVED $name   <-- TEST GAP"
  fi
}

run_mutation "print format %.1f -> %.2f" \
  'and %.1f bathrooms\n\0"' 'and %.2f bathrooms\n\0"'
run_mutation "print format %d -> %u for floors" \
  'b"The house has %d floors' 'b"The house has %u floors'
run_mutation "bathrooms += 1.0 -> += 2.0" \
  'store(h, OFF_BATHROOMS, v + 1.0)' 'store(h, OFF_BATHROOMS, v + 2.0)'
run_mutation "floors++ saturating instead of wrapping" \
  'v.wrapping_add(1)' 'v.saturating_add(1)'
run_mutation "bedrooms += saturating instead of wrapping" \
  'v.wrapping_add(extra_bedrooms)' 'v.saturating_add(extra_bedrooms)'
run_mutation "range check >= INT_MIN -> > INT_MIN" \
  'get_errno() == 0 && tmp >= INT_MIN' 'get_errno() == 0 && tmp > INT_MIN'
run_mutation "range check <= INT_MAX -> < INT_MAX" \
  'tmp <= INT_MAX {' 'tmp < INT_MAX {'
run_mutation "drop the errno check" \
  'get_errno() == 0 &&' 'true &&'
run_mutation "drop the endp != str check" \
  'endp != str_ as *mut c_char &&' 'true &&'
run_mutation "reject trailing garbage (over-strict)" \
  'endp != str_ as *mut c_char &&' 'endp.read() == 0 &&'
run_mutation "error message text" \
  'b"An error occurred\n\0"' 'b"An error occurred.\n\0"'
run_mutation "initial bathrooms 2.5 -> 2.0" \
  'bathrooms: 2.5,' 'bathrooms: 2.0,'
run_mutation "initial bedrooms 5 -> 6" \
  'bedrooms: 5,' 'bedrooms: 6,'
run_mutation "driver calls run once instead of twice" \
  '        run(&mut the_house as *mut house_t, x);
        run(&mut the_house as *mut house_t, x);' '        run(&mut the_house as *mut house_t, x);'
run_mutation "swap first print_house/add_floor in run" \
  '    print_house(h);
    add_floor(h);' '    add_floor(h);
    print_house(h);'
run_mutation "long->int narrowing altered" \
  'store(val as usize, 0, tmp as c_int);' 'store(val as usize, 0, (tmp >> 1) as c_int);'
run_mutation "strtol base 10 -> 16" \
  'strtol(str_, &mut endp, 10)' 'strtol(str_, &mut endp, 16)'
run_mutation "errno not cleared before strtol" \
  '    set_errno(0);
    let mut endp' '    let mut endp'
run_mutation "field offset floors/bedrooms swapped" \
  'const OFF_FLOORS: usize = offset_of!(house_t, floors);' \
  'const OFF_FLOORS: usize = offset_of!(house_t, bedrooms);'
run_mutation "bathrooms offset wrong" \
  'const OFF_BATHROOMS: usize = offset_of!(house_t, bathrooms);' \
  'const OFF_BATHROOMS: usize = 4;'

cp "$GOOD" src/lib.rs
cargo build -q >/dev/null 2>&1
echo "restored src/lib.rs sha256=$(sha256sum src/lib.rs | cut -c1-16)"
