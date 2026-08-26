#!/bin/bash
# Harness self-test: deliberately break src/lib.rs and confirm the differential
# suite catches it. Restores src/lib.rs afterwards. Run from the crate root.
#
#   ./mutation_check.sh
#
# Every mutant must be reported as CAUGHT. A SURVIVED mutant means the test
# suite has a blind spot (unless the mutant is provably ABI-equivalent, which is
# recorded in VERIFICATION.md).
set -u

ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
trap 'cp "$ORIG" src/lib.rs; rm -f "$ORIG"' EXIT

total=0
caught=0
survived_list=()
equiv_total=0
equiv_ok=0
equiv_bad=()
EXPECT_EQUIVALENT=0

mutate() {
  local desc="$1" old="$2" new="$3"
  if [ "$EXPECT_EQUIVALENT" -eq 1 ]; then
    equiv_total=$((equiv_total + 1))
  else
    total=$((total + 1))
  fi
  cp "$ORIG" src/lib.rs
  if ! python3 -c '
import sys
old, new = sys.argv[1], sys.argv[2]
s = open("src/lib.rs").read()
if old not in s:
    sys.exit("pattern not found: " + repr(old))
open("src/lib.rs", "w").write(s.replace(old, new, 1))
' "$old" "$new"; then
    echo "  ?? PATTERN-FAIL  $desc"
    return
  fi

  if ! timeout 300 cargo build >/dev/null 2>&1; then
    echo "  -- BUILD-FAIL    $desc (mutant does not compile; skipped)"
    if [ "$EXPECT_EQUIVALENT" -eq 1 ]; then
      equiv_total=$((equiv_total - 1))
    else
      total=$((total - 1))
    fi
    return
  fi

  # A mutant SURVIVES only if the suite reports a full clean sweep. Anything
  # else (assertion failure, abort, signal, timeout) counts as caught.
  local out plain_ok hardened_ok
  out=$(timeout 600 cargo test 2>&1)
  plain_ok=$(echo "$out" | grep -cE '^test result: ok\. 40 passed')
  out=$(MALLOC_CHECK_=3 MALLOC_PERTURB_=165 timeout 600 cargo test 2>&1)
  hardened_ok=$(echo "$out" | grep -cE '^test result: ok\. 40 passed')

  local was_caught=0
  if [ "$plain_ok" -eq 0 ] || [ "$hardened_ok" -eq 0 ]; then
    was_caught=1
  fi

  if [ "$EXPECT_EQUIVALENT" -eq 1 ]; then
    if [ "$was_caught" -eq 0 ]; then
      equiv_ok=$((equiv_ok + 1))
      echo "  == EQUIVALENT     $desc (survives, as expected)"
    else
      equiv_bad+=("$desc")
      echo "  ?? NOT-EQUIVALENT $desc (expected to survive but was caught)"
    fi
    return
  fi

  if [ "$was_caught" -eq 1 ]; then
    caught=$((caught + 1))
    local how="plain run"
    [ "$plain_ok" -ne 0 ] && how="hardened-allocator run only"
    echo "  OK CAUGHT ($how)  $desc"
  else
    survived_list+=("$desc")
    echo "  !! SURVIVED       $desc"
  fi
}

echo "=== mutation self-test of the differential suite ==="

# --- allocation-size arithmetic -------------------------------------------
mutate "nbytes: +4 -> +1 (under-allocates; heap overflow)" \
       "wrapping_add(4)" "wrapping_add(1)"
mutate "nbytes: *4 -> *3 (wrong growth factor)" \
       "wrapping_mul(4)" "wrapping_mul(3)"
mutate "nbytes: /3 -> /4" \
       "wrapping_div(3)" "wrapping_div(4)"
mutate "nbytes int->size_t: sign-extend -> zero-extend" \
       "nbytes as isize as usize" "nbytes as u32 as usize"
mutate "calloc -> malloc (tail no longer zero-filled)" \
       "calloc(core::mem::size_of::<c_char>(), nbytes as isize as usize)" \
       "{ unsafe extern \"C\" { fn malloc(size: usize) -> *mut c_void; } malloc(nbytes as isize as usize) }"

# --- input validation / control flow --------------------------------------
mutate "reject negative sizes (a 'sanitizing fix')" \
       "if src.is_null() {" \
       "if size < 0 { return core::ptr::null_mut(); }
    if src.is_null() {"
mutate "null check removed for size==0" \
       "if src.is_null() {" "if src.is_null() && size != 0 {"
mutate "strlen path: size == 0 -> size <= 0" \
       "if size == 0 {" "if size <= 0 {"
mutate "strlen result off by one" \
       "strlen(src) } as c_int" "strlen(src) + 1 } as c_int"
mutate "loop bound: i < size -> i <= size" \
       "while i < size {" "while i <= size {"
mutate "loop step 3 -> 2" \
       "i = i.wrapping_add(3);" "i = i.wrapping_add(2);"
mutate "padding branch: i+1 < size -> i+1 <= size" \
       "if i.wrapping_add(1) < size {
                *p = encode(b6);" \
       "if i.wrapping_add(1) <= size {
                *p = encode(b6);"
mutate "padding branch: i+2 < size -> i+2 <= size" \
       "if i.wrapping_add(2) < size {
                *p = encode(b7);" \
       "if i.wrapping_add(2) <= size {
                *p = encode(b7);"
mutate "b2 read guard: i+1 < size -> i+1 <= size" \
       "if i.wrapping_add(1) < size {
            b2 =" \
       "if i.wrapping_add(1) <= size {
            b2 ="

# --- bit twiddling --------------------------------------------------------
mutate "b4: b1 >> 2 -> b1 >> 3" "let b4: u8 = b1 >> 2;" "let b4: u8 = b1 >> 3;"
mutate "b5 mask: b1 & 0x3 -> b1 & 0x7" "((b1 & 0x3) << 4)" "((b1 & 0x7) << 4)"
mutate "b5 shift: b2 >> 4 -> b2 >> 5" "(b2 >> 4)" "(b2 >> 5)"
mutate "b6 mask: b2 & 0xf -> b2 & 0x7" "((b2 & 0xf) << 2)" "((b2 & 0x7) << 2)"
mutate "b6 shift: b3 >> 6 -> b3 >> 5" "(b3 >> 6)" "(b3 >> 5)"
mutate "b7 mask: b3 & 0x3f -> b3 & 0x1f" "b3 & 0x3f" "b3 & 0x1f"
mutate "signed char conversion dropped (sign bit masked off)" \
       "b1 = unsafe { *src.offset(i as isize) } as u8;" \
       "b1 = (unsafe { *src.offset(i as isize) } as u8) & 0x7f;"
mutate "b2 default 0 -> 0xff" "let mut b2: u8 = 0;" "let mut b2: u8 = 0xff;"
mutate "b3 default 0 -> 0xff" "let mut b3: u8 = 0;" "let mut b3: u8 = 0xff;"

# --- the base64 alphabet --------------------------------------------------
mutate "encode: u < 26 -> u <= 26" "if u < 26 {" "if u <= 26 {"
mutate "encode: u < 52 -> u < 51" "if u < 52 {" "if u < 51 {"
mutate "encode: u < 62 -> u < 63" "if u < 62 {" "if u < 63 {"
mutate "encode: '+' -> '-' (base64url)" "b'+' as c_char" "b'-' as c_char"
mutate "encode: '/' -> '_' (base64url)" "b'/' as c_char" "b'_' as c_char"
mutate "encode: 'a' + (u-26) -> 'a' + (u-25)" \
       "u.wrapping_sub(26)" "u.wrapping_sub(25)"
mutate "encode: '0' + (u-52) -> '0' + (u-53)" \
       "u.wrapping_sub(52)" "u.wrapping_sub(53)"
mutate "padding byte '=' -> 'A' (first)" \
       "*p = b'=' as c_char;" "*p = b'A' as c_char;"

# --- mutants that are PROVABLY ABI-equivalent -----------------------------
# These must survive: they change nothing an FFI caller can observe. They are
# listed so the reasoning is recorded and re-checked, not silently assumed.
echo
echo "--- provably ABI-equivalent mutants (expected to survive) ---"
EXPECT_EQUIVALENT=1
# calloc(1, n) and calloc(n, 1) request the same product and both fail the same
# way when n is huge, so glibc cannot distinguish them.
mutate "calloc args swapped (nmemb <-> size)" \
       "calloc(core::mem::size_of::<c_char>(), nbytes as isize as usize)" \
       "calloc(nbytes as isize as usize, core::mem::size_of::<c_char>())"
# trunc-div vs floor-div differ only for a negative dividend, i.e. only for
# negative `size`, where the loop never runs and the buffer is all zeros. The
# difference is at most 1 byte of allocation and never flips NULL-ness, so no
# FFI-observable behaviour changes. (Proof: see VERIFICATION.md.)
mutate "nbytes: C truncating division -> floor division" \
       "wrapping_div(3)" "div_euclid(3)"
EXPECT_EQUIVALENT=0

echo
echo "=== mutation self-test summary ==="
echo "behaviour-changing mutants caught: $caught/$total"
echo "ABI-equivalent mutants that correctly survived: $equiv_ok/$equiv_total"
rc=0
if [ ${#survived_list[@]} -gt 0 ]; then
  echo "SURVIVORS (blind spots in the test suite!):"
  for s in "${survived_list[@]}"; do echo "  - $s"; done
  rc=1
fi
if [ ${#equiv_bad[@]} -gt 0 ]; then
  echo "UNEXPECTEDLY CAUGHT (test may be flaky or the equivalence proof is wrong):"
  for s in "${equiv_bad[@]}"; do echo "  - $s"; done
  rc=1
fi
[ "$rc" -eq 0 ] && echo "All mutants behaved as expected."
exit $rc
