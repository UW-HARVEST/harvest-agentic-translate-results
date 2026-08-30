#!/usr/bin/env bash
# Mutation-sensitivity check for the differential test suite.
#
# Injects one deliberate bug at a time into src/lib.rs, rebuilds the cdylib and
# confirms the differential tests FAIL. A mutation that the suite does not catch
# is a blind spot in the tests. Restores the original source on exit.
set -u
cd "$(dirname "$0")"
export CARGO_NET_OFFLINE=true

BAK="$(mktemp -p . lib.rs.orig.XXXXXX)"
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; rm -f "$BAK"; cargo build --release >/dev/null 2>&1; }
trap restore EXIT

# name|python-replacement (old -> new)
mutate() { # $1 = description, $2 = old, $3 = new
    cp "$BAK" src/lib.rs
    python3 - "$2" "$3" <<'PY'
import sys
old, new = sys.argv[1], sys.argv[2]
s = open('src/lib.rs').read()
assert old in s, f"mutation pattern not found: {old!r}"
open('src/lib.rs','w').write(s.replace(old, new, 1))
PY
    [ $? -eq 0 ] || { echo "SKIP  $1 (pattern not found)"; return 1; }
    if ! cargo build --release >/dev/null 2>&1; then
        echo "SKIP  $1 (mutant did not compile)"; return 1
    fi
    n=$(cargo test --release 2>&1 | grep -cE '^test .* FAILED')
    if [ "$n" -gt 0 ]; then
        echo "CAUGHT  $1  ($n failing tests)"
    else
        echo "MISSED  $1  <-- BLIND SPOT: the suite accepted a wrong translation"
    fi
}

echo "== baseline =="
cp "$BAK" src/lib.rs
cargo build --release >/dev/null 2>&1
echo "baseline failing tests: $(cargo test --release 2>&1 | grep -cE '^test .* FAILED')"

echo "== mutants =="
mutate "uppercase hex (%02X instead of %02x)"      'b"%02x\0"'                       'b"%02X\0"'
mutate "no zero padding (%2x instead of %02x)"     'b"%02x\0"'                       'b"%2x\0"'
mutate "byte order reversed (big-endian dump)"     '*p.offset(i as isize)'           '*p.offset((len - 1 - i) as isize)'
mutate "sign-extended byte (i8 promotion)"         '*p.offset(i as isize) as c_int'  '(*p.offset(i as isize) as i8) as c_int'
mutate "off-by-one length (3 bytes)"               'core::mem::size_of::<f32>() as c_int' '(core::mem::size_of::<f32>() - 1) as c_int'
mutate "off-by-one length (5 bytes)"               'core::mem::size_of::<f32>() as c_int' '(core::mem::size_of::<f32>() + 1) as c_int'
mutate "missing trailing newline"                  'printf(b"\n\0".as_ptr() as *const c_char);' '/* newline dropped */'
mutate "newline replaced by \\r\\n"                'b"\n\0".as_ptr()'                'b"\r\n\0".as_ptr()'
mutate "NaN quietened (canonicalised argument)"    'let x = x;'                      'let x = if x.is_nan() { f32::NAN } else { x };'
mutate "negative zero flattened"                   'let x = x;'                      'let x = if x == 0.0 { 0.0 } else { x };'
mutate "subnormals flushed to zero"                'let x = x;'                      'let x = if x != 0.0 && x.abs() < f32::MIN_POSITIVE { 0.0f32.copysign(x) } else { x };'
mutate "loop uses unsigned compare"                'while i < len {'                 'while (i as u32) < (len as u32) {'
mutate "prints as f64 bytes"                       '&x as *const f32 as *const c_uchar' '&(x as f64) as *const f64 as *const c_uchar'
