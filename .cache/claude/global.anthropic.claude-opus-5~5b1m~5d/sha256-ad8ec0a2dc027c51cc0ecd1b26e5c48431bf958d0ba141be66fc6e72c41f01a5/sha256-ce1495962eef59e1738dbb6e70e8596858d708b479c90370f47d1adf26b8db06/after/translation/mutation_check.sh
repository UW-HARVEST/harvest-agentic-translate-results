#!/usr/bin/env bash
# Test-power check ("who tests the tests?"): inject a bug into the Rust
# translation, one at a time, and confirm the differential suite CATCHES it.
# A mutation that survives unexpectedly means the suite has a blind spot.
#
# Each entry is  name|sed-expression|expectation  where expectation is
#   catch         the suite must fail in every profile
#   equivalent    provably behaviour-preserving, so surviving is correct
#   dev-only      only observable with debug-assertions (Rust UB checks) on
#   release-only  only observable at opt-level > 0 (LLVM eliding UB-dependent
#                 memory traffic)
#
# Usage: ./mutation_check.sh                (dev profile)
#        PROFILE=--release ./mutation_check.sh
set -u
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK=target/lib.rs.orig
PROFILE=${PROFILE:-}
mkdir -p target
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

MUTATIONS=(
  # ---- dispatch / control flow -------------------------------------------
  "arity dispatch boundary (len<2 -> len<3)|s/if len < 2 {/if len < 3 {/|catch"
  "arity drops the unsigned-char truncation|s/let len: u8 = (len as u32 \& 0xff) as u8;/let len: i64 = len as i64;/|catch"
  "arity len==2 never matches|s/} else if len == 2 {/} else if len == 99 {/|catch"
  "arity len==3 never matches|s/} else if len == 3 {/} else if len == 98 {/|catch"
  # ---- apply_bitmask ------------------------------------------------------
  "apply_bitmask mask1 0xF0 -> 0xF1|s/let mask1: c_int = 0b11110000;/let mask1: c_int = 0b11110001;/|catch"
  "apply_bitmask mask2 0x0F -> 0x1F|s/let mask2: c_int = 0b00001111;/let mask2: c_int = 0b00011111;/|catch"
  "apply_bitmask mask3 OR -> AND|s/2 => value | mask3,/2 => value \& mask3,/|catch"
  "apply_bitmask mask4 XOR -> OR|s/3 => value ^ mask4,/3 => value | mask4,/|catch"
  "apply_bitmask default returns 0|s/_ => value,/_ => 0,/|catch"
  # ---- shift_array -------------------------------------------------------
  "shift_array guard positions<size -> <=|s/if positions > 0 \&\& positions < size {/if positions > 0 \&\& positions <= size {/|catch"
  "shift_array guard positions>0 -> >=0|s/if positions > 0 \&\& positions < size {/if positions >= 0 \&\& positions < size {/|catch"
  "shift_array zero-fills positions+1 slots|s/while i < positions {/while i <= positions {/|catch"
  "shift_array moves size instead of size-positions elements|s/let elems = size.wrapping_sub(positions) as isize;/let elems = size as isize;/|catch"
  # ---- arity4 arithmetic --------------------------------------------------
  "arity4 uses Euclidean modulo instead of C remainder|s/param1.wrapping_rem(4)/param1.rem_euclid(4)/|catch"
  "arity4 divides by 101 instead of 100|s/wrapping_div(100)/wrapping_div(101)/|catch"
  "arity4 reads matrix[2][2] instead of [2][3]|s/.wrapping_add(matrix\[2\]\[3\])/.wrapping_add(matrix[2][2])/|catch"
  "arity4 shifts by 2 instead of 1|s/shift_array(block.values.as_mut_ptr(), 4, 1)/shift_array(block.values.as_mut_ptr(), 4, 2)/|catch"
  "arity4 param3 guard != 0 -> > 0|s/if param3 != 0 {/if param3 > 0 {/|catch"
  "arity4 param4 guard != 0 -> > 0|s/if param4 != 0 {/if param4 > 0 {/|catch"
  "arity4 sums only 3 of the 4 block values|s/while i < block.count {/while i < block.count - 1 {/|catch"
  # ---- init_matrix / process_string --------------------------------------
  "init_matrix last element 12 -> 13|s/\[9, 10, 11, 12\]/[9, 10, 11, 13]/|catch"
  "init_matrix writes column-major|s/store(matrix.add(i \* 4 + j), temp\[i\]\[j\]);/store(matrix.add(j * 3 + i), temp[i][j]);/|catch"
  "process_string guard inverted|s/if unsafe { load(str) } != 0 {/if unsafe { load(str) } == 0 {/|catch"
  # ---- compare_allocations ----------------------------------------------
  "compare_allocations bonus test > 0 -> >= 0|s/if unsafe { load(uninit_ptr) } > 0 {/if unsafe { load(uninit_ptr) } >= 0 {/|catch"
  "compare_allocations bonus 10 -> 11|s/^        10$/        11/|catch"
  "compare_allocations malloc-failure returns 0 instead of -1|s/        return -1;/        return 0;/|catch"
  "compare_allocations equal-pointer branch returns 4 instead of 3|s/result = 3;/result = 4;/|catch"
  "compare_allocations bonus reads val1 instead of the stored memory|s/unsafe { load(uninit_ptr) } > 0/val1 > 0/|catch"
  "compare_allocations swaps the < and > branches|s/if (ptr1 as usize) < (ptr2 as usize) {/if (ptr1 as usize) > (ptr2 as usize) {/|catch"
  # ---- wrappers ----------------------------------------------------------
  "arity3 forwards param4=1 instead of 0|s/arity4(p1, p2, p3, 0)/arity4(p1, p2, p3, 1)/|catch"
  "arity2 forwards param3=1 instead of 0|s/arity4(p1, p2, 0, 0)/arity4(p1, p2, 1, 0)/|catch"
  # ---- memory-access fidelity (profile sensitive) ------------------------
  "load via plain deref (adds a NULL check under debug-assertions)|s|unsafe { read_volatile(p as \*const Unaligned<T>).0 }|unsafe { *p }|;|dev-only"
  "load via read_volatile (adds an alignment check under debug-assertions)|s|unsafe { read_volatile(p as \*const Unaligned<T>).0 }|unsafe { read_volatile(p) }|;|dev-only"
  "store via plain write (adds a NULL check under debug-assertions)|s|unsafe { write_volatile(p as \*mut Unaligned<T>, Unaligned(v)) }|unsafe { *p = v }|;|dev-only"
  "compare_allocations plain deref + no black_box (optimiser elides store+reload)|s|        store(ptr1, val1);|        *ptr1 = val1;|; s|        store(ptr2, val2);|        *ptr2 = val2;|; s|unsafe { load(uninit_ptr) } > 0|unsafe { *uninit_ptr } > 0|; s|core::hint::black_box(unsafe { malloc(core::mem::size_of::<c_int>()) } as \*mut c_int)|(unsafe { malloc(core::mem::size_of::<c_int>()) } as *mut c_int)|g;|release-only"
  "EQUIVALENT: no black_box (redundant while the packed volatile access is kept)|s|core::hint::black_box(unsafe { malloc(core::mem::size_of::<c_int>()) } as \*mut c_int)|(unsafe { malloc(core::mem::size_of::<c_int>()) } as *mut c_int)|g;|equivalent"
  # ---- provably behaviour-preserving: surviving is the correct outcome ---
  "EQUIVALENT: process_string always calls strlen (strlen(\"\")==0 anyway)|s/if unsafe { load(str) } != 0 {/if true {/|equivalent"
  "EQUIVALENT: signed pointer compare (no x86-64 user address has bit 63 set)|s/if (ptr1 as usize) < (ptr2 as usize) {/if (ptr1 as isize) < (ptr2 as isize) {/|equivalent"
)

is_release=0
[ "$PROFILE" = "--release" ] && is_release=1

caught=0; ok_survivors=0; bad_count=0
BAD=""

echo "profile: ${PROFILE:-dev}"
printf '%-78s %s\n' "MUTATION" "RESULT"
printf '%.0s-' {1..115}; printf '\n'

for entry in "${MUTATIONS[@]}"; do
  name=${entry%%|*}
  rest=${entry#*|}
  expr=${rest%|*}
  expect=${rest##*|}

  # Resolve profile-sensitive expectations.
  case "$expect" in
    dev-only)     if [ $is_release -eq 1 ]; then expect=equivalent; else expect=catch; fi ;;
    release-only) if [ $is_release -eq 1 ]; then expect=catch; else expect=equivalent; fi ;;
  esac

  restore
  sed -i "$expr" "$SRC"
  if cmp -s "$SRC" "$BAK"; then
    printf '%-78s %s\n' "$name" "!! SKIPPED (sed pattern did not match)"
    bad_count=$((bad_count+1)); BAD="$BAD\n  - $name -> sed pattern did not match"; continue
  fi
  if ! cargo build $PROFILE >target/mut-build.log 2>&1; then
    printf '%-78s %s\n' "$name" "!! SKIPPED (does not compile)"
    bad_count=$((bad_count+1)); BAD="$BAD\n  - $name -> does not compile"; continue
  fi
  if cargo test $PROFILE --tests >target/mut-test.log 2>&1; then
    if [ "$expect" = equivalent ]; then
      printf '%-78s %s\n' "$name" "survived (expected)"
      ok_survivors=$((ok_survivors+1))
    else
      printf '%-78s %s\n' "$name" "*** SURVIVED - BLIND SPOT ***"
      bad_count=$((bad_count+1)); BAD="$BAD\n  - $name -> survived but should have been caught"
    fi
  else
    failing=$(grep -hoE '^test [a-z0-9_]+ \.\.\. FAILED' target/mut-test.log \
              | sed 's/^test //; s/ \.\.\. FAILED//' | head -3 | tr '\n' ' ')
    n=$(grep -cE '^test [a-z0-9_]+ \.\.\. FAILED' target/mut-test.log || true)
    if [ "$expect" = equivalent ]; then
      printf '%-78s %s\n' "$name" "!! FAILED although expected to survive: $failing"
      bad_count=$((bad_count+1)); BAD="$BAD\n  - $name -> failed but expected to survive"
    else
      if [ "$n" -eq 0 ]; then
        printf '%-78s %s\n' "$name" "caught (test binary aborted - see target/mut-test.log)"
      else
        printf '%-78s %s\n' "$name" "caught by $n test(s): ${failing:-<see log>}"
      fi
      caught=$((caught+1))
    fi
  fi
done

restore
cargo build $PROFILE >/dev/null 2>&1

printf '%.0s-' {1..115}; printf '\n'
echo "caught: $caught   expected survivors: $ok_survivors   problems: $bad_count"
if [ "$bad_count" -ne 0 ]; then
  printf 'PROBLEMS:%b\n' "$BAD"; exit 1
fi
echo "OK: every non-equivalent injected bug was detected by the differential suite."
