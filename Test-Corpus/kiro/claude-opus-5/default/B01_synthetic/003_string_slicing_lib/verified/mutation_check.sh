#!/usr/bin/env bash
# Negative control / mutation testing for the differential suite.
#
# A test suite that passes proves nothing unless it can also FAIL. This injects
# small behaviour changes into src/lib.rs one at a time and asserts the suite
# catches each one, then restores the original source.
set -uo pipefail
cd "$(dirname "$0")"

BAK=$(mktemp)
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap restore EXIT

C_SO="$(cd .. && pwd)/c_src/build/libString_Slice.so"
OVERALL=0

run_suite() {
    timeout 600 cargo build >/dev/null 2>&1 || return 2
    SLICE_RUST_SO="$PWD/target/debug/libString_Slice.so" RUST_TEST_THREADS=1 \
        timeout 600 cargo test --no-fail-fast -- --test-threads=1 > /tmp/mutation.log 2>&1
}

mutate() {
    local desc="$1"; shift
    restore
    "$@" || { echo "  !! mutation command failed for: $desc"; OVERALL=1; return; }
    if ! grep -q . src/lib.rs; then echo "  !! empty lib.rs"; OVERALL=1; return; fi

    if run_suite; then
        echo "NOT CAUGHT (BAD): $desc"
        OVERALL=1
    else
        n=$(grep -cE '\.\.\. FAILED$' /tmp/mutation.log)
        first=$(grep -E '\.\.\. FAILED$' /tmp/mutation.log | head -3 | tr '\n' ' ')
        echo "caught: $desc  ($n failing tests: $first)"
    fi
    restore
}

echo "== baseline must PASS"
if run_suite; then echo "baseline ok"; else echo "BASELINE FAILS - fix that first"; exit 1; fi

echo
echo "== injected mutations (each must be CAUGHT)"

mutate "start check uses >= instead of > (off-by-one at start==len)" \
    sed -i 's/if (start as usize) > len {/if (start as usize) >= len {/' src/lib.rs

mutate "start compared as signed (drops the C's unsigned promotion)" \
    sed -i 's/if (start as usize) > len {/if (start as i64) > (len as i64) {/' src/lib.rs

mutate "stop check uses >= instead of >" \
    sed -i 's/if (stop as usize) > len {/if (stop as usize) >= len {/' src/lib.rs

mutate "stop compared as signed (drops the C's unsigned promotion)" \
    sed -i 's/if (stop as usize) > len {/if (stop as i64) > (len as i64) {/' src/lib.rs

mutate "ordering check uses < instead of <= (allows stop==start)" \
    sed -i 's/if stop <= start {/if stop < start {/' src/lib.rs

mutate "printed width off by one" \
    sed -i 's/stop.wrapping_sub(start),/stop.wrapping_sub(start).wrapping_sub(1),/' src/lib.rs

mutate "slice offset off by one" \
    sed -i 's/mystr.offset(start as isize)/mystr.offset(start as isize + 1)/' src/lib.rs

mutate "error return value 1 -> -1 on the start branch" \
    python3 -c "
import re,io
p='src/lib.rs'; s=open(p).read()
s=s.replace('''printf(ERR_START.as_ptr() as *const c_char);
                return 1;''','''printf(ERR_START.as_ptr() as *const c_char);
                return -1;''')
open(p,'w').write(s)"

mutate "swap the order of the two stop checks" \
    python3 -c "
p='src/lib.rs'; s=open(p).read()
a='''            if (stop as usize) > len {
                printf(ERR_STOP_OFF_END.as_ptr() as *const c_char);
                return 1;
            }
            // C: signed comparison.
            if stop <= start {
                printf(ERR_STOP_ORDER.as_ptr() as *const c_char);
                return 1;
            }'''
b='''            if stop <= start {
                printf(ERR_STOP_ORDER.as_ptr() as *const c_char);
                return 1;
            }
            if (stop as usize) > len {
                printf(ERR_STOP_OFF_END.as_ptr() as *const c_char);
                return 1;
            }'''
assert a in s, 'pattern not found'
open(p,'w').write(s.replace(a,b))"

mutate "swap the two error messages" \
    python3 -c "
p='src/lib.rs'; s=open(p).read()
s=s.replace('printf(ERR_STOP_OFF_END','printf(ERR_STOP_ORDER_TMP').replace('printf(ERR_STOP_ORDER.','printf(ERR_STOP_OFF_END.').replace('printf(ERR_STOP_ORDER_TMP','printf(ERR_STOP_ORDER')
open(p,'w').write(s)"

mutate "default stop becomes len-1 instead of len when stop_ptr is NULL" \
    sed -i 's/stop = len as c_int;/stop = len as c_int - 1;/' src/lib.rs

mutate "default start becomes 1 instead of 0 when start_ptr is NULL" \
    sed -i 's/^            start = 0;$/            start = 1;/' src/lib.rs

mutate "add a NULL check on mystr that the C does not have" \
    python3 -c "
p='src/lib.rs'; s=open(p).read()
s=s.replace('        let len: usize = strlen(mystr);','        if mystr.is_null() { return 1; }\n        let len: usize = strlen(mystr);')
open(p,'w').write(s)"

mutate "trailing newline dropped from the printed slice" \
    python3 -c "
p='src/lib.rs'; s=open(p).read()
q=chr(34); bs=chr(92)
a=q+'%.*s'+bs+'n'+bs+'0'+q
b=q+'%.*s'+bs+'0'+q
assert a in s, 'FMT_SLICE literal not found'
open(p,'w').write(s.replace(a,b))"

mutate "write the clamped index back through start_ptr (C never writes)" \
    python3 -c "
p='src/lib.rs'; s=open(p).read()
s=s.replace('            start = *start_ptr;','            start = *start_ptr;\n            *start_ptr = 12345;')
open(p,'w').write(s)"

restore
echo
if [ "$OVERALL" -eq 0 ]; then
    echo "ALL MUTATIONS CAUGHT — the differential suite is sensitive."
else
    echo "SOME MUTATIONS WERE NOT CAUGHT — the suite has blind spots."
fi
# Confirm the tree is clean again.
diff -q "$BAK" src/lib.rs && echo "src/lib.rs restored to the verified original."
exit "$OVERALL"
