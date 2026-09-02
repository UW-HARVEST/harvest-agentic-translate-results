#!/usr/bin/env bash
# Mutation test: deliberately inject bugs into the Rust translation, rebuild the
# .so, and confirm the differential suite CATCHES each one. A mutation that
# survives means the tests are blind to that class of bug.
#
# Every mutation is checked against BOTH the release and the debug Rust .so,
# because some divergences (rustc's debug-assertions null-pointer check) only
# manifest in one profile.
#
# Usage: ./mutation_check.sh      (run from translation/)
set -u

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap 'restore; rm -f "$BAK"' EXIT

# Mutations that are provably EQUIVALENT to the original (no input distinguishes
# them through the exported ABI), so surviving is the correct outcome, not a
# test gap.
#
# Group 1 - clamp-threshold comparisons:
#   validate lower bound `<` -> `<=` : differs only at value == 0100 (64), where
#       the original falls through and returns `value` (64) while the mutant
#       returns `lower_threshold` (64). Same number.
#   validate upper bound `>` -> `>=` : differs only at value == 0777 (511),
#       where the original returns `value` (511) and the mutant returns
#       `upper_threshold` (511). Same number.
#   Verified, not argued: tests/exhaustive.rs
#   `exhaustive_validate_and_normalize_full_i32` compares all 2^32 inputs of
#   validate_and_normalize against the C and they agree on every single one.
#
# Group 2 - dead stores inside findrep. `grep -n 'message\|result' c_src/src/lib.c`
#   shows `message` is written at lib.c:122, mutated at lib.c:148, copied to
#   `final_message` at lib.c:151, and NEVER read again; `result` is only ever
#   written from `search_buffer`'s 'p' offset (lib.c:127), the four operation
#   returns, the accumulator/multiplier term, operation_count and the sentinel.
#   So nothing about the message text is observable through any exported symbol:
#     process_octal_string literal 0o123 -> decimal 123
#     findrep replaces the wrong char in message
#   Likewise `search_buffer` only contributes the INDEX of its first 'p' (9), so
#   editing the literal after that 'p' changes nothing:
#     octal string search literal changed after the 'p'
#   The sharper variant below ("...moves the 'p'") edits the literal BEFORE the
#   'p' and MUST be caught, which is what proves the tests are sensitive here.
EQUIVALENT_MUTANTS="validate lower bound < -> <=|validate upper bound > -> >=|process_octal_string literal 0o123 -> decimal 123|findrep replaces the wrong char in message|octal string search literal changed after the 'p'"

pass=0
fail=0
equiv=0
skip=0

# Build both profiles and run the suite against each; returns 0 if BOTH pass.
run_suite_both_profiles() {
  timeout 300 cargo build --release >/dev/null 2>&1 || return 2
  timeout 300 cargo build          >/dev/null 2>&1 || return 2
  FINDREP_RUST_SO="$PWD/target/release/libfindrep_lib.so" \
    timeout 600 cargo test --release >/dev/null 2>&1 || return 1
  FINDREP_RUST_SO="$PWD/target/debug/libfindrep_lib.so" \
    timeout 600 cargo test --release >/dev/null 2>&1 || return 1
  return 0
}

run_mutation() {
  local name="$1"; shift
  restore
  if ! perl -0pi -e "$1" "$SRC" 2>/dev/null; then
    echo "SKIP  $name (patch errored)"; skip=$((skip+1)); return
  fi
  if cmp -s "$SRC" "$BAK"; then
    echo "SKIP  $name (patch matched nothing)"; skip=$((skip+1)); return
  fi
  local is_equiv=0
  case "|$EQUIVALENT_MUTANTS|" in
    *"|$name|"*) is_equiv=1 ;;
  esac

  run_suite_both_profiles
  local rc=$?
  case "$rc" in
    2) echo "SKIP  $name (does not compile)"; skip=$((skip+1)) ;;
    0) if [ "$is_equiv" -eq 1 ]; then
         echo "EQUIVALENT  $name   (expected to survive)"; equiv=$((equiv+1))
       else
         echo "SURVIVED    $name   <-- TEST GAP"; fail=$((fail+1))
       fi ;;
    *) if [ "$is_equiv" -eq 1 ]; then
         echo "UNEXPECTED  $name was killed but is supposed to be equivalent"; fail=$((fail+1))
       else
         echo "caught      $name"; pass=$((pass+1))
       fi ;;
  esac
}

# --- octal-literal constants -----------------------------------------------
run_mutation "accumulator gate 0o150 -> decimal 150" \
  's/if ACCUMULATOR > 0o150/if ACCUMULATOR > 150/'
run_mutation "multiplier gate 0o100 -> decimal 100" \
  's/if MULTIPLIER > 0o100/if MULTIPLIER > 100/'
run_mutation "operation_count weight 0o10 -> decimal 10" \
  's/OPERATION_COUNT\.wrapping_mul\(0o10\)/OPERATION_COUNT.wrapping_mul(10)/'
run_mutation "sentinel 0o777 -> decimal 777" \
  's/        result = 0o777;/        result = 777;/'
run_mutation "validate lower_threshold 0o100 -> decimal 100" \
  's/let lower_threshold: c_int = 0o100;/let lower_threshold: c_int = 100;/'
run_mutation "validate upper_threshold 0o777 -> decimal 777" \
  's/let upper_threshold: c_int = 0o777;/let upper_threshold: c_int = 777;/'
run_mutation "process_octal_string literal 0o123 -> decimal 123" \
  's/process_octal_string\(message\.as_mut_ptr\(\), 0o123\)/process_octal_string(message.as_mut_ptr(), 123)/'

# --- control flow ----------------------------------------------------------
run_mutation "drop the result==0 sentinel entirely" \
  's/    if result_exists == 0 \{\n        result = 0o777;\n    \}//'
run_mutation "validate lower bound < -> <=" \
  's/if value < lower_threshold/if value <= lower_threshold/'
run_mutation "validate upper bound > -> >=" \
  's/\} else if value > upper_threshold/} else if value >= upper_threshold/'
run_mutation "validate clamps non-positive too (drop value > 0)" \
  's/if is_nonzero != 0 && value > 0/if is_nonzero != 0/'
run_mutation "mode_add gate 0o1 -> 0o0" \
  's/let mode_add: c_int = 0o1;/let mode_add: c_int = 0o0;/'
run_mutation "mode_multiply gate 0o2 -> 0o3" \
  's/let mode_multiply: c_int = 0o2;/let mode_multiply: c_int = 0o3;/'
run_mutation "both_active uses || instead of &&" \
  's/\(has_accumulator != 0 && has_multiplier != 0\)/(has_accumulator != 0 || has_multiplier != 0)/'
run_mutation "accumulator gate > -> >=" \
  's/if ACCUMULATOR > 0o150/if ACCUMULATOR >= 0o150/'
run_mutation "multiplier gate > -> >=" \
  's/if MULTIPLIER > 0o100/if MULTIPLIER >= 0o100/'

# --- state ----------------------------------------------------------------
run_mutation "MULTIPLIER init 1 -> 0" \
  's/static mut MULTIPLIER: c_int = 1;/static mut MULTIPLIER: c_int = 0;/'
run_mutation "ACCUMULATOR init 0 -> 1" \
  's/static mut ACCUMULATOR: c_int = 0;/static mut ACCUMULATOR: c_int = 1;/'
run_mutation "OPERATION_COUNT init 0 -> 1" \
  's/static mut OPERATION_COUNT: c_int = 0;/static mut OPERATION_COUNT: c_int = 1;/'
run_mutation "operation_count not incremented in add" \
  's/    ACCUMULATOR = ACCUMULATOR\.wrapping_add\(a\.wrapping_add\(b\)\);\n    OPERATION_COUNT = OPERATION_COUNT\.wrapping_add\(1\);/    ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_add(b));/'
run_mutation "subtract uses + instead of -" \
  's/ACCUMULATOR\.wrapping_sub\(a\.wrapping_sub\(b\)\)/ACCUMULATOR.wrapping_sub(a.wrapping_add(b))/'
run_mutation "add uses - instead of +" \
  's/ACCUMULATOR\.wrapping_add\(a\.wrapping_add\(b\)\)/ACCUMULATOR.wrapping_add(a.wrapping_sub(b))/'
run_mutation "multiply uses + instead of *" \
  's/MULTIPLIER\.wrapping_mul\(a\.wrapping_mul\(b\)\)/MULTIPLIER.wrapping_mul(a.wrapping_add(b))/'
run_mutation "operations table order swapped (add<->subtract)" \
  's/static OPERATIONS: \[OperationFunc; 4\] = \[\n    add_to_accumulator,\n    multiply_with_multiplier,\n    subtract_from_accumulator,/static OPERATIONS: [OperationFunc; 4] = [\n    subtract_from_accumulator,\n    multiply_with_multiplier,\n    add_to_accumulator,/'
run_mutation "findrep divide called with b=3 instead of 2" \
  's/selected_op\(MULTIPLIER, 2\);/selected_op(MULTIPLIER, 3);/'
run_mutation "findrep memchr offset off by one" \
  's/result = result\.wrapping_add\(offset as c_int\);/result = result.wrapping_add(offset as c_int + 1);/'

# --- division -------------------------------------------------------------
run_mutation "divide_multiplier drops the b != 0 guard" \
  's/    if b != 0 \{\n(\s*\/\/[^\n]*\n)*\s*MULTIPLIER = c_idiv\(MULTIPLIER, b\);\n    \}/    if b != 0 { MULTIPLIER = c_idiv(MULTIPLIER, b); } else { MULTIPLIER = 0; }/'
run_mutation "divide_multiplier uses a instead of b as divisor" \
  's/pub unsafe extern "C" fn divide_multiplier\(_a: c_int, b: c_int\)/pub unsafe extern "C" fn divide_multiplier(b: c_int, _a: c_int)/'
run_mutation "c_idiv -> wrapping_div (loses the SIGFPE trap)" \
  's/    MULTIPLIER = c_idiv\(MULTIPLIER, b\);/    MULTIPLIER = MULTIPLIER.wrapping_div(b);/'
run_mutation "c_idiv returns the remainder instead of the quotient" \
  's/        out\("edx"\) _,/        out("edx") quotient2,/;
   s/    quotient\n\}/    let _ = quotient; quotient2\n}/;
   s/    let quotient: c_int;/    let quotient: c_int; let quotient2: c_int;/'

# --- string / memory helpers ---------------------------------------------
run_mutation "memchr compares without truncating to unsigned char" \
  's/if core::ptr::read\(haystack\.add\(i\)\) as u8 == target/if (core::ptr::read(haystack.add(i)) as c_int) == needle/'
run_mutation "memchr returns LAST match instead of first" \
  's/    for i in 0\.\.len \{\n        if core::ptr::read\(haystack\.add\(i\)\) as u8 == target \{\n            return Some\(i\);\n        \}\n    \}\n    None/    let mut last = None;\n    for i in 0..len {\n        if core::ptr::read(haystack.add(i)) as u8 == target { last = Some(i); }\n    }\n    last/'
run_mutation "memchr scans len+1 bytes (includes the terminator)" \
  's/    for i in 0\.\.len \{\n        if core::ptr::read\(haystack\.add\(i\)\)/    for i in 0..=len {\n        if core::ptr::read(haystack.add(i))/'
run_mutation "replacement char X -> Y" \
  "s/core::ptr::write\\(s\\.add\\(idx\\), b'X' as c_char\\)/core::ptr::write(s.add(idx), b'Y' as c_char)/"
run_mutation "process_octal_string omits the NUL terminator" \
  's/    core::ptr::write\(dest\.add\(src\.len\(\)\), 0\);//'
run_mutation "strlen off by one" \
  's/    while core::ptr::read\(s\.add\(n\)\) != 0 \{\n        n \+= 1;\n    \}\n    n/    while core::ptr::read(s.add(n)) != 0 { n += 1; }\n    n.saturating_sub(1)/'
run_mutation "%o formatted as signed instead of unsigned" \
  's/format!\("\{:o\}", v as u32\)/format!("{:o}", v as i64)/'
run_mutation "octal string drops the leading 0 prefix" \
  's/"Octal: 0\{\}, Decimal: \{\}"/"Octal: {}, Decimal: {}"/'
run_mutation "octal string search literal changed after the 'p'" \
  's/b"Function pointer example with static vars"/b"Function pointer example with static var"/'
run_mutation "octal string search literal changed so it moves the 'p'" \
  's/b"Function pointer example with static vars"/b"unction pointer example with static vars"/'
run_mutation "findrep replaces the wrong char in message" \
  "s/find_and_replace_char\\(message\\.as_mut_ptr\\(\\), b'O' as c_int\\)/find_and_replace_char(message.as_mut_ptr(), b'0' as c_int)/"

# --- profile-sensitive: raw deref instead of ptr::read/write --------------
# Reintroduces rustc's debug-assertions null-pointer check, which makes the
# DEBUG .so abort (SIGABRT) where the C segfaults (SIGSEGV). Must be caught.
run_mutation "raw deref in c_strlen (debug null-check divergence)" \
  's/    while core::ptr::read\(s\.add\(n\)\) != 0 \{/    while *s.add(n) != 0 {/'
run_mutation "raw deref in c_strcpy_bytes (debug null-check divergence)" \
  's/        core::ptr::write\(dest\.add\(i\), \*b as c_char\);/        *dest.add(i) = *b as c_char;/;
   s/    core::ptr::write\(dest\.add\(src\.len\(\)\), 0\);/    *dest.add(src.len()) = 0;/'

restore
cargo build --release >/dev/null 2>&1
cargo build >/dev/null 2>&1

echo
echo "=== mutation summary: $pass caught, $equiv equivalent-by-design, $fail survived, $skip skipped ==="
[ "$fail" -eq 0 ]
