#!/usr/bin/env bash
# Negative control: the differential suite is only meaningful if it FAILS when
# the Rust diverges from the C. Inject known bugs into src/lib.rs one at a time,
# confirm the suite catches each, then restore the original.
#
# NOTE: `cargo build` is mandatory before `cargo test` -- cargo does not rebuild
# a `crate-type = ["cdylib"]` artifact for the test profile, so without it the
# suite would dlopen a stale .so and pass unconditionally.
set -u
cd "$(dirname "$0")"

cp src/lib.rs /tmp/lib.rs.orig
trap 'cp /tmp/lib.rs.orig src/lib.rs; cargo build -q >/dev/null 2>&1' EXIT

fail=0
apply() {
  python3 - "$1" "$2" <<'PY'
import sys, pathlib
frm, to = sys.argv[1], sys.argv[2]
p = pathlib.Path("src/lib.rs")
s = p.read_text()
if frm not in s:
    sys.exit("MUTANT ANCHOR NOT FOUND: %r" % frm)
p.write_text(s.replace(frm, to, 1))
PY
}

# mutate <expect: KILL|EQUIV> <name> <from> <to>
mutate() {
  local expect="$1" name="$2" from="$3" to="$4"
  cp /tmp/lib.rs.orig src/lib.rs
  if ! apply "$from" "$to"; then
    echo "ERROR       $name  (anchor missing -- mutant not applied)"
    fail=1
    return
  fi
  if ! timeout 600 cargo build -q >/tmp/mutbuild.log 2>&1; then
    echo "ERROR       $name  (mutant does not compile -- not a behavioural test)"
    sed -n '1,15p' /tmp/mutbuild.log
    fail=1
    return
  fi
  if timeout 600 cargo test -q >/tmp/mut.log 2>&1; then
    if [ "$expect" = "EQUIV" ]; then
      echo "equivalent  $name  (survives as predicted -- see comment)"
    else
      echo "NOT CAUGHT  $name  <-- suite is blind to this bug"
      fail=1
    fi
  else
    local n
    n=$(grep -c 'DIVERGENCE' /tmp/mut.log)
    if [ "$expect" = "EQUIV" ]; then
      echo "NOT EQUIVALENT $name  <-- predicted to survive but was killed ($n reports)"
      fail=1
    elif [ "$n" -eq 0 ]; then
      echo "caught      $name  (via an oracle assertion, 0 DIVERGENCE reports)"
    else
      echo "caught      $name  ($n DIVERGENCE reports)"
    fi
  fi
}

echo "== negative control: injecting known bugs =="

mutate KILL "M1  drop the hex_pos-- on a trailing odd nibble" \
  'hex_pos = hex_pos.wrapping_sub(1);' \
  '/* mutant: no decrement */'

mutate KILL "M2  u8 instead of u32 arithmetic for c_num0" \
  'let c_num0: u8 = ((c_num as u32).wrapping_sub(10) >> 8) as u8;' \
  'let c_num0: u8 = ((c_num.wrapping_sub(10)) as u32 >> 8) as u8;'

mutate KILL "M3  u8 instead of u32 arithmetic for c_alpha0" \
  'let c_alpha0: u8 = (((c_alpha as u32).wrapping_sub(10)
            ^ (c_alpha as u32).wrapping_sub(16))
            >> 8) as u8;' \
  'let c_alpha0: u8 = (((c_alpha.wrapping_sub(10) ^ c_alpha.wrapping_sub(16)) as u32) >> 8) as u8;'

mutate KILL "M4  fix the strchr NUL-terminator quirk (treat NUL as not-found)" \
  'let b = unsafe { *p } as u8;
        if b == c {' \
  'let b = unsafe { *p } as u8;
        if b == 0 { return false; }
        if b == c {'

# M5 is a PROVABLY EQUIVALENT mutant, kept as an equivalence assertion:
# deferring the `bin_pos >= bin_maxlen` test from the high nibble to the next
# (low) nibble is unobservable, because (a) bin_pos cannot change in between, so
# the predicate has the same value at both points; (b) if it fires one iteration
# later, hex_pos has advanced by 1 but state is now 0xFF, so the `state != 0`
# epilogue decrements hex_pos back to exactly the index the C reports; (c) no
# byte is written to bin on either path and ret is -1 on both. The same
# cancellation holds when the high nibble is the last byte and when the next byte
# is non-hex or ignorable. Verified empirically by tests/exhaustive.rs.
mutate EQUIV "M5  defer the buffer-full check to the low nibble (equivalent)" \
  'if bin_pos >= bin_maxlen {
            ret = -1;
            break;
        }

        if state == 0u8 {' \
  'if state != 0u8 && bin_pos >= bin_maxlen {
            ret = -1;
            break;
        }

        if state == 0u8 {'

mutate KILL "M6  honour the ignore set at odd nibbles too" \
  'if !ignore.is_null() && state == 0u8 && unsafe { c_strchr_found(ignore, c) } {' \
  'if !ignore.is_null() && unsafe { c_strchr_found(ignore, c) } {'

mutate KILL "M7  drop the 'unconsumed input with NULL hex_end_p is an error' branch" \
  'if !hex_end_p.is_null() {
        // *hex_end_p = &hex[hex_pos];
        unsafe { *hex_end_p = hex.wrapping_add(hex_pos) };
    } else if hex_pos != hex_len {
        ret = -1;
    }' \
  'if !hex_end_p.is_null() {
        unsafe { *hex_end_p = hex.wrapping_add(hex_pos) };
    }'

mutate KILL "M8  point hex_end one past instead of at the unpaired digit" \
  'unsafe { *hex_end_p = hex.wrapping_add(hex_pos) };' \
  'unsafe { *hex_end_p = hex.wrapping_add(hex_pos.wrapping_add(1)) };'

mutate KILL "M9  forget to scale the high nibble by 16" \
  'c_acc = c_val.wrapping_mul(16);' \
  'c_acc = c_val;'

mutate KILL "M10 accept high-bit aliases of A-F (clear bit 7 as well as bit 5)" \
  'let c_alpha: u8 = ((c as u32 & !32u32).wrapping_sub(55)) as u8;' \
  'let c_alpha: u8 = ((c as u32 & !160u32).wrapping_sub(55)) as u8;'

mutate KILL "M11 return bin_pos instead of the -1 sentinel on error" \
  'if ret != 0 {
        return ret;
    }
    bin_pos as c_int' \
  'let _ = ret;
    bin_pos as c_int'

mutate KILL "M12 off-by-one: allow one byte past bin_maxlen" \
  'if bin_pos >= bin_maxlen {' \
  'if bin_pos > bin_maxlen {'

cp /tmp/lib.rs.orig src/lib.rs
echo "== restoring original and re-verifying =="
if timeout 600 cargo build -q >/tmp/mutbuild.log 2>&1 && timeout 600 cargo test -q >/tmp/mut.log 2>&1; then
  echo "original source: suite PASSES"
else
  echo "original source: suite FAILS -- restoration broke something"
  tail -30 /tmp/mut.log
  fail=1
fi

if [ "$fail" -ne 0 ]; then echo "NEGATIVE CONTROL FAILED"; exit 1; fi
echo "NEGATIVE CONTROL OK: every injected bug was caught"
