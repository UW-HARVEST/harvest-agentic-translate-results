#!/usr/bin/env bash
# Negative control for the differential suite.
#
# Injects known bugs into src/lib.rs, one at a time, and asserts the test suite
# FAILS for each. A suite that stays green under mutation is not actually
# comparing anything. src/lib.rs is always restored afterwards.
set -u
cd "$(dirname "$0")" || exit 1

BAK="${TMPDIR:-/tmp}/lib.rs.orig"
cp src/lib.rs "$BAK" || exit 1
restore() { cp "$BAK" src/lib.rs; }
trap restore EXIT

rc_overall=0

try_mutation() {
    local name="$1"; shift
    restore
    "$@" || { echo "SKIP  $name (could not apply)"; return; }
    if cmp -s "$BAK" src/lib.rs; then
        echo "SKIP  $name (mutation was a no-op)"
        return
    fi
    local out
    # `cargo build` first: cargo test does NOT rebuild a cdylib-only lib target,
    # so without this the suite would load the previous (unmutated) .so.
    out=$(timeout 300 cargo build --release --offline 2>&1
          timeout 300 cargo test --release --offline 2>&1)
    if echo "$out" | grep -qE '^test result: FAILED|error\[E'; then
        local n
        n=$(echo "$out" | grep -cE 'FAILED$')
        echo "CAUGHT $name  ($n test(s) failed)"
    else
        echo "MISSED $name  <-- suite did not detect this bug"
        rc_overall=1
    fi
    restore
}

m_no_pad()   { sed -i 's/b"%02x\\0"/b"%2x\\0"/'                                    src/lib.rs; }
m_signed()   { sed -i 's/\*p\.offset(i as isize) as c_int/(*p.offset(i as isize) as i8) as c_int/' src/lib.rs; }
m_uppercase(){ sed -i 's/b"%02x\\0"/b"%02X\\0"/'                                   src/lib.rs; }
m_len3()     { sed -i 's/core::mem::size_of::<c_int>() as c_int/3/'                src/lib.rs; }
m_len5()     { sed -i 's/core::mem::size_of::<c_int>() as c_int/5/'                src/lib.rs; }
m_no_nl()    { sed -i 's/^    printf(b"\\n\\0".as_ptr() as \*const c_char);/    \/\/ removed newline/' src/lib.rs; }
m_bigendian(){ sed -i 's/\*p\.offset(i as isize)/*p.offset((len - 1 - i) as isize)/' src/lib.rs; }
m_off_by_one(){ sed -i 's/while i < len {/while i < len - 1 {/'                     src/lib.rs; }

echo "=== mutation testing src/lib.rs against the differential suite ==="
try_mutation "%2x (no zero padding)"        m_no_pad
try_mutation "signed char promotion"        m_signed
try_mutation "%02X (uppercase hex)"         m_uppercase
try_mutation "len = 3 (truncated dump)"     m_len3
try_mutation "len = 5 (over-read)"          m_len5
try_mutation "missing trailing newline"     m_no_nl
try_mutation "big-endian byte order"        m_bigendian
try_mutation "loop off-by-one"              m_off_by_one

restore
echo "=== src/lib.rs restored ==="
if cmp -s "$BAK" src/lib.rs; then echo "restore verified: identical to original"
else echo "ERROR: restore failed!"; rc_overall=1; fi
exit $rc_overall
