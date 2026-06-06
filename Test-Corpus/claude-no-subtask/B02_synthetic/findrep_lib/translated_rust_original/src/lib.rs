// Translation of c_src/src/lib.c to Rust.
// Preserves the exact behavior of the C source, including static/global state.

use std::ffi::c_int;

// Static state mirrors the C `static int` globals.
static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_add(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    unsafe {
        MULTIPLIER = MULTIPLIER.wrapping_mul(a.wrapping_mul(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_sub(a.wrapping_sub(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    unsafe {
        if b != 0 {
            // C signed division truncates toward zero; matches Rust integer `/`.
            // Use wrapping_div to mirror C UB-like behavior on INT_MIN / -1
            // without panicking in release mode (Rust's `/` would panic in
            // debug). For non-edge-case inputs, behavior is identical to `/`.
            MULTIPLIER = MULTIPLIER.wrapping_div(b);
        }
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero: c_int = if value != 0 { 1 } else { 0 };
    let _is_zero: c_int = if value == 0 { 1 } else { 0 };

    let lower_threshold: c_int = 0o100; // 64
    let upper_threshold: c_int = 0o777; // 511

    if is_nonzero != 0 && value > 0 {
        if value < lower_threshold {
            return lower_threshold;
        } else if value > upper_threshold {
            return upper_threshold;
        }
    }

    value
}

// Function-pointer table mirroring the C `operations[4]` array.
type OperationFunc = fn(c_int, c_int) -> c_int;
const OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

#[unsafe(no_mangle)]
pub extern "C" fn findrep(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let p1_valid: c_int = if param1 != 0 { 1 } else { 0 };
    let p2_valid: c_int = if param2 != 0 { 1 } else { 0 };
    let p3_valid: c_int = if param3 != 0 { 1 } else { 0 };
    let p4_valid: c_int = if param4 != 0 { 1 } else { 0 };

    let active_params: c_int = p1_valid
        .wrapping_add(p2_valid)
        .wrapping_add(p3_valid)
        .wrapping_add(p4_valid);

    let mode_add: c_int = 0o1;
    let mode_multiply: c_int = 0o2;
    // mode_subtract / mode_divide are declared in the C source but never
    // checked against `active_params`; we keep them only as dead bindings
    // for parity (they have no observable effect).
    let _mode_subtract: c_int = 0o3;
    let _mode_divide: c_int = 0o4;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    // The C source builds two character buffers, but only one of them
    // contributes to `result`: the offset of the first 'p' in
    // "Function pointer example with static vars". memchr returns the
    // pointer to that byte, and `(int)(found_char - search_buffer)` is
    // exactly its index. The other buffer ("message") is mutated locally
    // and never observed in the return value, so we omit those operations
    // for parity-on-return-value (the function has no other outputs).
    let search_buffer: &[u8] = b"Function pointer example with static vars";
    if let Some(pos) = search_buffer.iter().position(|&c| c == b'p') {
        result = result.wrapping_add(pos as c_int);
    }

    let mut selected_op: OperationFunc;

    if active_params >= mode_add {
        selected_op = OPERATIONS[0];
        result = result.wrapping_add(selected_op(normalized_p1, normalized_p2));
    }

    if active_params >= mode_multiply {
        selected_op = OPERATIONS[1];
        result = result.wrapping_add(selected_op(normalized_p3, normalized_p4));
    }

    let acc_snapshot = unsafe { ACCUMULATOR };
    if acc_snapshot > 0o150 {
        selected_op = OPERATIONS[2];
        let subtract_result = selected_op(normalized_p1, normalized_p3);
        result = result.wrapping_add(subtract_result);
    }

    // find_and_replace_char(message, 'O') — mutates a local buffer that is
    // never read again. No effect on return value, so skipped.

    let acc_now = unsafe { ACCUMULATOR };
    let mul_now = unsafe { MULTIPLIER };
    let has_accumulator: bool = acc_now != 0;
    let has_multiplier: bool = mul_now != 0;
    let both_active: bool = has_accumulator && has_multiplier;

    if both_active {
        result = result.wrapping_add(acc_now.wrapping_add(mul_now));
    }

    let mul_check = unsafe { MULTIPLIER };
    if mul_check > 0o100 {
        selected_op = OPERATIONS[3];
        let _ = selected_op(mul_check, 2);
    }

    let op_count = unsafe { OPERATION_COUNT };
    result = result.wrapping_add(op_count.wrapping_mul(0o10));

    let result_exists: c_int = if result != 0 { 1 } else { 0 };
    if result_exists == 0 {
        result = 0o777;
    }

    result
}
