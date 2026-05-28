// Translation of c_src/src/lib.c to Rust.
// Preserves the C behavior including mutable static state.

use std::ffi::c_int;
use std::os::raw::c_char;

// Static globals matching the C `static int` declarations.
static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

type OperationFunc = unsafe extern "C" fn(c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_add(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    unsafe {
        MULTIPLIER = MULTIPLIER.wrapping_mul(a.wrapping_mul(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_sub(a.wrapping_sub(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    unsafe {
        if b != 0 {
            MULTIPLIER = MULTIPLIER.wrapping_div(b);
        }
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

/// Writes the C string "Octal: 0<oct>, Decimal: <dec>\0" to `dest`.
/// Mirrors `sprintf` then `strcpy` from the C implementation.
unsafe fn process_octal_string_internal(dest: *mut c_char, octal_val: c_int) {
    // C %o for a positive int prints the octal representation without a
    // leading zero. The format string in C is "Octal: 0%o, Decimal: %d".
    // For negative values, C %o would print the value reinterpreted as
    // unsigned int; reproduce the same via cast to u32.
    let octal_str = format!("{:o}", octal_val as u32);
    let s = format!("Octal: 0{}, Decimal: {}", octal_str, octal_val);
    let bytes = s.as_bytes();
    unsafe {
        for (i, b) in bytes.iter().enumerate() {
            *dest.add(i) = *b as c_char;
        }
        *dest.add(bytes.len()) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    unsafe { process_octal_string_internal(dest, octal_val) }
}

/// Computes the C strlen of `s`.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut len = 0usize;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    len
}

/// Performs memchr equivalent: returns a pointer to first occurrence of
/// `byte` in the first `len` bytes of `s`, or null if not found.
unsafe fn c_memchr(s: *const c_char, byte: u8, len: usize) -> *const c_char {
    unsafe {
        for i in 0..len {
            if *s.add(i) as u8 == byte {
                return s.add(i);
            }
        }
    }
    std::ptr::null()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_and_replace_char(s: *mut c_char, search_char: c_int) {
    unsafe {
        let len = c_strlen(s);
        // C: memchr(str, search_char, strlen(str)) — search_char is an int
        // but only the low byte is used.
        let target = (search_char as u32) as u8;
        let found = c_memchr(s as *const c_char, target, len);
        if !found.is_null() {
            // Cast away const-ness since the original buffer is mutable.
            let found_mut = found as *mut c_char;
            *found_mut = b'X' as c_char;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn validate_and_normalize(value: c_int) -> c_int {
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

// Static dispatch table mirroring `static operation_func operations[4]`.
static OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn findrep(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
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
        let _mode_subtract: c_int = 0o3;
        let _mode_divide: c_int = 0o4;

        let normalized_p1 = validate_and_normalize(param1);
        let normalized_p2 = validate_and_normalize(param2);
        let normalized_p3 = validate_and_normalize(param3);
        let normalized_p4 = validate_and_normalize(param4);

        let mut message: [c_char; 100] = [0; 100];
        let mut search_buffer: [c_char; 100] = [0; 100];

        process_octal_string_internal(message.as_mut_ptr(), 0o123);

        // strcpy(search_buffer, "Function pointer example with static vars")
        let src = b"Function pointer example with static vars";
        for (i, b) in src.iter().enumerate() {
            search_buffer[i] = *b as c_char;
        }
        search_buffer[src.len()] = 0;

        let buf_len = c_strlen(search_buffer.as_ptr());
        let found_char = c_memchr(search_buffer.as_ptr(), b'p', buf_len);
        if !found_char.is_null() {
            let offset = (found_char as isize) - (search_buffer.as_ptr() as isize);
            result = result.wrapping_add(offset as c_int);
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

        if ACCUMULATOR > 0o150 {
            selected_op = OPERATIONS[2];
            let subtract_result = selected_op(normalized_p1, normalized_p3);
            result = result.wrapping_add(subtract_result);
        }

        find_and_replace_char(message.as_mut_ptr(), b'O' as c_int);

        // strcpy(final_message, message) — final_message is unused but copy
        // for side-effect parity (no-op observably).
        let mut final_message: [c_char; 100] = [0; 100];
        let msg_len = c_strlen(message.as_ptr());
        for i in 0..=msg_len {
            final_message[i] = message[i];
        }
        let _ = final_message;

        let has_accumulator: c_int = if ACCUMULATOR != 0 { 1 } else { 0 };
        let has_multiplier: c_int = if MULTIPLIER != 0 { 1 } else { 0 };
        let both_active: c_int = if has_accumulator != 0 && has_multiplier != 0 {
            1
        } else {
            0
        };

        if both_active != 0 {
            result = result
                .wrapping_add(ACCUMULATOR)
                .wrapping_add(MULTIPLIER);
        }

        if MULTIPLIER > 0o100 {
            selected_op = OPERATIONS[3];
            selected_op(MULTIPLIER, 2);
        }

        result = result.wrapping_add(OPERATION_COUNT.wrapping_mul(0o10));

        let result_exists: c_int = if result != 0 { 1 } else { 0 };
        if result_exists == 0 {
            result = 0o777;
        }

        result
    }
}
