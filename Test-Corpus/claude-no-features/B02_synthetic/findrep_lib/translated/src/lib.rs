// Translated from c_src/src/lib.c
// Preserves C semantics including non-thread-safe static state.

use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

// Static variables matching the C globals.
static ACCUMULATOR: AtomicI32 = AtomicI32::new(0);
static MULTIPLIER: AtomicI32 = AtomicI32::new(1);
static OPERATION_COUNT: AtomicI32 = AtomicI32::new(0);

fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    let cur = ACCUMULATOR.load(Ordering::Relaxed);
    let new_val = cur.wrapping_add(a.wrapping_add(b));
    ACCUMULATOR.store(new_val, Ordering::Relaxed);
    OPERATION_COUNT.fetch_add(1, Ordering::Relaxed);
    new_val
}

fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    let cur = MULTIPLIER.load(Ordering::Relaxed);
    let new_val = cur.wrapping_mul(a.wrapping_mul(b));
    MULTIPLIER.store(new_val, Ordering::Relaxed);
    OPERATION_COUNT.fetch_add(1, Ordering::Relaxed);
    new_val
}

fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    let cur = ACCUMULATOR.load(Ordering::Relaxed);
    let new_val = cur.wrapping_sub(a.wrapping_sub(b));
    ACCUMULATOR.store(new_val, Ordering::Relaxed);
    OPERATION_COUNT.fetch_add(1, Ordering::Relaxed);
    new_val
}

fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    if b != 0 {
        let cur = MULTIPLIER.load(Ordering::Relaxed);
        // C signed integer division truncates toward zero.
        let new_val = cur.wrapping_div(b);
        MULTIPLIER.store(new_val, Ordering::Relaxed);
    }
    OPERATION_COUNT.fetch_add(1, Ordering::Relaxed);
    MULTIPLIER.load(Ordering::Relaxed)
}

// Build the formatted "Octal: 0%o, Decimal: %d" message into `dest` as a
// byte vector mirroring the C string written by sprintf+strcpy.
fn process_octal_string(dest: &mut Vec<u8>, octal_val: c_int) {
    // Format octal without the leading '0' prefix (the literal '0' is added
    // explicitly), matching C's printf %o conversion.
    // The C cast in printf is to unsigned; replicate that.
    let unsigned_val = octal_val as u32;
    let formatted = format!("Octal: 0{:o}, Decimal: {}", unsigned_val, octal_val);
    dest.clear();
    dest.extend_from_slice(formatted.as_bytes());
}

// Replicates C: memchr finds the first occurrence of `search_char` (as a
// byte) within the first strlen(str) bytes; if found, replace with 'X'.
fn find_and_replace_char(buf: &mut [u8], search_char: c_int) {
    let needle = search_char as u8;
    if let Some(pos) = buf.iter().position(|&b| b == needle) {
        buf[pos] = b'X';
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

// Operation dispatch table mirroring C's `static operation_func operations[4]`.
fn dispatch_operation(idx: usize, a: c_int, b: c_int) -> c_int {
    match idx {
        0 => add_to_accumulator(a, b),
        1 => multiply_with_multiplier(a, b),
        2 => subtract_from_accumulator(a, b),
        3 => divide_multiplier(a, b),
        _ => unreachable!(),
    }
}

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

    let active_params: c_int = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add: c_int = 0o1;
    let mode_multiply: c_int = 0o2;
    let _mode_subtract: c_int = 0o3;
    let _mode_divide: c_int = 0o4;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let _normalized_p4 = validate_and_normalize(param4);
    // Note: `_normalized_p4` retained to mirror the C variable; used below.
    let normalized_p4 = _normalized_p4;

    // Buffers — sized to match C's char[100].
    let mut message: Vec<u8> = Vec::with_capacity(100);
    let search_buffer: &[u8] = b"Function pointer example with static vars";

    process_octal_string(&mut message, 0o123); // 0o123 == 83 decimal

    // memchr equivalent on search_buffer.
    let found_char_offset: Option<usize> =
        search_buffer.iter().position(|&b| b == b'p');
    if let Some(off) = found_char_offset {
        result = result.wrapping_add(off as c_int);
    }

    if active_params >= mode_add {
        let r = dispatch_operation(0, normalized_p1, normalized_p2);
        result = result.wrapping_add(r);
    }

    if active_params >= mode_multiply {
        let r = dispatch_operation(1, normalized_p3, normalized_p4);
        result = result.wrapping_add(r);
    }

    if ACCUMULATOR.load(Ordering::Relaxed) > 0o150 {
        let subtract_result = dispatch_operation(2, normalized_p1, normalized_p3);
        result = result.wrapping_add(subtract_result);
    }

    find_and_replace_char(&mut message, b'O' as c_int);

    // final_message = strcpy(message); — created but unused beyond copy in C.
    let _final_message: Vec<u8> = message.clone();

    let has_accumulator: c_int =
        if ACCUMULATOR.load(Ordering::Relaxed) != 0 { 1 } else { 0 };
    let has_multiplier: c_int =
        if MULTIPLIER.load(Ordering::Relaxed) != 0 { 1 } else { 0 };
    let both_active: c_int = if has_accumulator != 0 && has_multiplier != 0 {
        1
    } else {
        0
    };

    if both_active != 0 {
        let acc = ACCUMULATOR.load(Ordering::Relaxed);
        let mul = MULTIPLIER.load(Ordering::Relaxed);
        result = result.wrapping_add(acc.wrapping_add(mul));
    }

    if MULTIPLIER.load(Ordering::Relaxed) > 0o100 {
        // selected_op = divide_multiplier; selected_op(multiplier, 2);
        let mul_val = MULTIPLIER.load(Ordering::Relaxed);
        dispatch_operation(3, mul_val, 2);
    }

    result = result.wrapping_add(
        OPERATION_COUNT.load(Ordering::Relaxed).wrapping_mul(0o10),
    );

    let result_exists: c_int = if result != 0 { 1 } else { 0 };
    if result_exists == 0 {
        result = 0o777;
    }

    result
}
