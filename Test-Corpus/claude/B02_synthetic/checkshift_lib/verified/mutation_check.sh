#!/usr/bin/env bash
# Harness validation ("do the tests actually discriminate?").
#
# Injects one deliberate divergence into the Rust source at a time, rebuilds the
# .so, runs the test that is SUPPOSED to catch it, and reports PASS only if the
# test FAILS. A mutation that the suite does not notice is a blind spot.
#
# The C source is never touched.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

BK=${TMPDIR:-/tmp}/mutation-backup
rm -rf "$BK"; mkdir -p "$BK"
cp -a src "$BK/src"
# NOTE: `cp -a` preserves mtimes, so a restored file looks OLDER than the .so
# cargo just built from the mutated source - cargo would then skip the rebuild
# and leave a mutated .so behind. Always bump the mtimes after restoring.
restore() {
  rm -rf src
  cp -a "$BK/src" src
  find src -type f -exec touch {} +
}
trap 'restore' EXIT

# Deliberately EXCLUDED as provably equivalent (a suite "missing" these is
# correct behaviour, not a blind spot):
#   * `(checksum << 1)` -> `checksum.rotate_left(1)`: at most 16 bytes are
#     folded, so the accumulator stays <= 0x007FFFFF (see the bound in
#     ERRORS.md/E12 notes) and bit 31 is never shifted out. Identical results.
#   * `!values.is_null() && count > 0` -> `count > 0 && !values.is_null()`:
#     both operands are side-effect free, so the reordering is a no-op.
#
# sed expressions must not contain a literal '|' (the field separator).
#
# name | file | sed-expr | test-filter | test-binary
mutations=(
  "shift: logical instead of arithmetic >>|src/ops.rs|s#(b >> shift)#((b as u32) >> shift) as c_int#|c9_shift_neg_b|phase_b_valid"
  "shift: wrong shift amount|src/ops.rs|s#static STATIC_SHIFT_AMOUNT: c_int = 2;#static STATIC_SHIFT_AMOUNT: c_int = 3;#|c7_shift_pos_pos|phase_b_valid"
  "multiply: wrong static multiplier|src/ops.rs|s#static STATIC_MULTIPLIER: c_int = 3;#static STATIC_MULTIPLIER: c_int = 4;#|c1_multiply_random|phase_b_valid"
  "multiply: saturating instead of wrapping|src/ops.rs|s#a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)#a.saturating_mul(b).saturating_mul(STATIC_MULTIPLIER)#|c2_multiply_edges|phase_b_valid"
  "add: wrong addend|src/ops.rs|s#static STATIC_ADDEND: c_int = 100;#static STATIC_ADDEND: c_int = 101;#|c3_add_random|phase_b_valid"
  "add: saturating instead of wrapping|src/ops.rs|s#a.wrapping_add(b).wrapping_add(STATIC_ADDEND)#a.saturating_add(b).saturating_add(STATIC_ADDEND)#|c4_add_edges|phase_b_valid"
  "xor: wrong constant|src/ops.rs|s#a ^ b ^ 0xABCD#a ^ b ^ 0xABCE#|c5_xor_random|phase_b_valid"
  "ops table: entries 0 and 1 swapped|src/ops.rs|s#Some(multiply_with_static as unsafe extern \"C\" fn(c_int, c_int) -> c_int),#Some(add_with_static as unsafe extern \"C\" fn(c_int, c_int) -> c_int),#|c11_get_operation_pointer_identity|phase_b_valid"
  "get_operation: off-by-one upper bound|src/ops.rs|s#if opcode >= 0 \&\& opcode < 4 {#if opcode >= 0 \&\& opcode <= 4 {#|e2_get_operation_opcode_at_and_past_upper_bound|phase_c_errors"
  "get_operation: accepts negatives|src/ops.rs|s#if opcode >= 0 \&\& opcode < 4 {#if opcode > -2 \&\& opcode < 4 {#|e1_get_operation_negative_opcode|phase_c_errors"
  "execute_operation: LOG_VALUE text changed|src/ops.rs|s#Variable a = %d#Variable A = %d#|c14_execute_operation_all_ops|phase_b_valid"
  "execute_operation: NULL diagnostic text changed|src/ops.rs|s#Error: Operation function pointer is NULL for %s#Error: operation function pointer is NULL for %s#|e4_execute_operation_null_func|phase_c_errors"
  "execute_operation: NULL sentinel 0 -> -1|src/ops.rs|s#^        return 0;#        return -1;#|e4_execute_operation_null_func|phase_c_errors"
  "execute_operation: %s rendered as %p|src/ops.rs|s#is NULL for %s#is NULL for %p#|e5_execute_operation_null_func_null_name|phase_c_errors"
  "execute_operation: logs b's value under a's label|src/ops.rs|s#print_i(b\"Variable a = %d\\\\n\\\\0\", a);#print_i(b\"Variable a = %d\\\\n\\\\0\", b);#|c14_execute_operation_all_ops|phase_b_valid"
  "execute_operation: ignores the supplied func|src/ops.rs|s#let result = unsafe { func(a, b) };#let result = unsafe { OPS[0].unwrap()(a, b) };#|c16_execute_operation_foreign_func|phase_b_valid"
  "checksum: shift by 2 instead of 1|src/state.rs|s#checksum = (checksum << 1) ^ c_uint::from(byte);#checksum = (checksum << 2) ^ c_uint::from(byte);#|c18_checksum_byte_patterns|phase_b_valid"
  "checksum: clamp at 3 instead of 4|src/state.rs|s#let copy_count = if count > 4 { 4 } else { count } as usize;#let copy_count = if count > 3 { 3 } else { count } as usize;#|c17_checksum_counts_1_to_4|phase_b_valid"
  "checksum: clamp at 2, seen through oversized counts|src/state.rs|s#let copy_count = if count > 4 { 4 } else { count } as usize;#let copy_count = if count > 2 { 2 } else { count } as usize;#|e12_compute_checksum_oversized_count_clamps|phase_c_errors"
  "checksum: wrong MAGIC_NUMBER|src/state.rs|s#const MAGIC_NUMBER: c_uint = 0xDEAD_BEEF;#const MAGIC_NUMBER: c_uint = 0xDEAD_BEEE;#|c17_checksum_counts_1_to_4|phase_b_valid"
  "checksum: wrong MASK_LOWER|src/state.rs|s#const MASK_LOWER: c_uint = 0x0000_FFFF;#const MASK_LOWER: c_uint = 0x0007_FFFF;#|c18_checksum_byte_patterns|phase_b_valid"
  "checksum: byteswap the fold (byte-order bug)|src/state.rs|s#for \&byte in \&buffer\[..byte_len\] {#for \&byte in buffer[..byte_len].iter().rev() {#|c17_checksum_counts_1_to_4|phase_b_valid"
  "checksum: accepts count == 0|src/state.rs|s#if !values.is_null() \&\& count > 0 {#if !values.is_null() \&\& count >= 0 {#|e9_compute_checksum_zero_count|phase_c_errors"
  "checksum: nonzero seed leaks into the rejected path|src/state.rs|s#let mut checksum: c_uint = 0;#let mut checksum: c_uint = 1;#|e8_compute_checksum_null_values|phase_c_errors"
  "checksum: negative count treated as valid|src/state.rs|s#if !values.is_null() \&\& count > 0 {#if !values.is_null() \&\& count != 0 {#|e10_compute_checksum_negative_count|phase_c_errors"
  "init_state: leaves checksum field alone|src/state.rs|s#checksum: 0x0000,#checksum: 0xFFFF,#|c19_init_state_fresh_and_dirty|phase_b_valid"
  "init_state: operation_count starts at 1|src/state.rs|s#operation_count: 0,#operation_count: 1,#|c19_init_state_fresh_and_dirty|phase_b_valid"
  "init_state: NULL diagnostic text changed|src/state.rs|s#Error: state pointer is NULL in init_state#Error: State pointer is NULL in init_state#|e13_init_state_null_state|phase_c_errors"
  "init_state: success message text changed|src/state.rs|s#State initialized with accumulator = %d#State initialised with accumulator = %d#|c19_init_state_fresh_and_dirty|phase_b_valid"
  "apply_operation: does not bump operation_count|src/state.rs|s#(\*state).operation_count = (\*state).operation_count.wrapping_add(1);#(*state).operation_count = (*state).operation_count;#|c20_apply_operation_single|phase_b_valid"
  "apply_operation: operand order swapped|src/state.rs|s#func((\*state).accumulator, value)#func(value, (*state).accumulator)#|c21_apply_operation_chains|phase_b_valid"
  "apply_operation: NULL-check precedence swapped|src/state.rs|s#Error: state pointer is NULL in apply_operation#Error: operation function pointer is NULL in apply_operation#|e16_apply_operation_null_state_and_func|phase_c_errors"
  "apply_operation: mutates state despite NULL func|src/state.rs|s#print_lit(b\"Error: operation function pointer is NULL in apply_operation\\\\n\\\\0\");#{ print_lit(b\"Error: operation function pointer is NULL in apply_operation\\\\n\\\\0\"); unsafe { (*state).operation_count += 1; } }#|e15_apply_operation_null_func|phase_c_errors"
  "checkshift: final fold uses signed xor|src/checkshift.rs|s#((\*state).accumulator.wrapping_add(shift_result) as u32 ^ (\*state).checksum) as c_int#(*state).accumulator.wrapping_add(shift_result) ^ ((*state).checksum as c_int) \& 0x7FFF#|c23_checkshift_random|phase_b_valid"
  "checkshift: checksum uses count 3|src/checkshift.rs|s#compute_checksum(params.as_mut_ptr(), 4)#compute_checksum(params.as_mut_ptr(), 3)#|c23_checkshift_random|phase_b_valid"
  "checkshift: checksum printed as %04x not %04X|src/checkshift.rs|s#Computed checksum: 0x%04X#Computed checksum: 0x%04x#|c23_checkshift_random|phase_b_valid"
  "checkshift: SHIFT gets param3 instead of param2|src/checkshift.rs|s#^            param2,\$#            param3,#|c24_checkshift_edges|phase_b_valid"
  "checkshift: XOR gets param3 instead of param4|src/checkshift.rs|s#^            param4,\$#            param3,#|c23_checkshift_random|phase_b_valid"
  "checkshift: failure sentinel -1 -> -2|src/checkshift.rs|s#        return -1;#        return -2;#|e17_checkshift_malloc_failure|phase_c_errors"
  "checkshift: alloc-failure diagnostic text changed|src/checkshift.rs|s#Error: Failed to allocate memory for state#Error: failed to allocate memory for state#|e17_checkshift_malloc_failure|phase_c_errors"
  "checkshift: continues past a failed allocation|src/checkshift.rs|s#    if state.is_null() {#    if false {#|e17_checkshift_malloc_failure|phase_c_errors"
  "init_state: NULL diagnostic reused in apply_operation|src/state.rs|s#Error: state pointer is NULL in apply_operation#Error: state ptr is NULL in apply_operation#|e14_apply_operation_null_state|phase_c_errors"
  "checkshift: params array reordered before checksum|src/checkshift.rs|s#let mut params: \[c_int; 4\] = \[param1, param2, param3, param4\];#let mut params: [c_int; 4] = [param1, param2, param4, param3];#|c23_checkshift_random|phase_b_valid"
  "checkshift: op 1 and 2 order swapped|src/checkshift.rs|s#unsafe { apply_operation(state, param2, mult_op) };#unsafe { apply_operation(state, param2, add_op) };#|c23_checkshift_random|phase_b_valid"
  "checkshift: banner text changed|src/checkshift.rs|s#=== Starting foo function ===#=== Starting checkshift function ===#|c23_checkshift_random|phase_b_valid"
  "checkshift: trailing blank line dropped|src/checkshift.rs|s#=== Ending foo function ===\\\\n\\\\n\\\\0#=== Ending foo function ===\\\\n\\\\0#|c23_checkshift_random|phase_b_valid"
  "checkshift: XOR op name changed|src/checkshift.rs|s#b\"XOR\\\\0\"#b\"Xor\\\\0\"#|c23_checkshift_random|phase_b_valid"
)

pass=0; fail=0
declare -a blind=()

for m in "${mutations[@]}"; do
  IFS='|' read -r name file expr filter bin <<< "$m"
  restore
  if ! sed -i "$expr" "$file" 2>/dev/null; then
    echo "SKIP  (sed error)          : $name"; continue
  fi
  if diff -q "$BK/$file" "$file" > /dev/null; then
    echo "SKIP  (mutation did not apply): $name"
    blind+=("NOT-APPLIED: $name")
    continue
  fi
  if ! timeout 300 cargo build --no-default-features > /dev/null 2>&1; then
    echo "SKIP  (does not compile)   : $name"
    continue
  fi
  if timeout 300 cargo test --no-default-features --test "$bin" \
        -- --exact "$filter" --test-threads=1 > /dev/null 2>&1; then
    echo "BLIND SPOT: $filter did NOT catch: $name"
    blind+=("$filter missed: $name")
    fail=$((fail+1))
  else
    echo "caught by $filter: $name"
    pass=$((pass+1))
  fi
done

restore
timeout 300 cargo build --no-default-features > /dev/null 2>&1

echo "======================================================"
echo "mutations caught: $pass    blind spots: ${#blind[@]}"
if ((${#blind[@]})); then
  printf '  %s\n' "${blind[@]}"
  exit 1
fi
echo "HARNESS VALIDATION: OK"
