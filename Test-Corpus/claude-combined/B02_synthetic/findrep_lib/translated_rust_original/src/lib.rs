// Rust translation of c_src/src/lib.c
// Preserves byte-identical behavior, including global mutable state.

use std::ffi::c_char;
use std::ffi::c_int;

// Match C's static file-scope globals.
static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_add(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    unsafe {
        MULTIPLIER = MULTIPLIER.wrapping_mul(a.wrapping_mul(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_sub(a.wrapping_sub(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    unsafe {
        if b != 0 {
            MULTIPLIER /= b;
        }
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

/// Equivalent of `sprintf(buffer, "Octal: 0%o, Decimal: %d", octal_val, octal_val); strcpy(dest, buffer);`
///
/// Caller must guarantee `dest` points to a buffer with at least 50 bytes
/// (the same constraint the C version imposes via its local `buffer[50]`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    // Replicate the printf %o behavior: print as unsigned octal without a prefix,
    // then prepend a '0' character ourselves (the format string does so).
    // %d uses signed decimal.
    let octal_unsigned = octal_val as u32; // %o uses unsigned conversion
    let formatted = format!("Octal: 0{:o}, Decimal: {}", octal_unsigned, octal_val);
    let bytes = formatted.as_bytes();
    unsafe {
        for (i, &b) in bytes.iter().enumerate() {
            *dest.add(i) = b as c_char;
        }
        *dest.add(bytes.len()) = 0; // null terminator
    }
}

/// Equivalent of:
///   char* found = (char*)memchr(str, search_char, strlen(str));
///   if (found) { *found = 'X'; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_and_replace_char(str_: *mut c_char, search_char: c_int) {
    if str_.is_null() {
        return;
    }
    // memchr searches the first `strlen(str)` bytes. If `search_char` does
    // not appear before the NUL, no replacement occurs (matches C since
    // memchr won't see the NUL).
    let needle = (search_char as u32 & 0xFF) as u8;
    unsafe {
        let mut i: usize = 0;
        loop {
            let c = *str_.add(i) as u8;
            if c == 0 {
                return;
            }
            if c == needle {
                *str_.add(i) = b'X' as c_char;
                return;
            }
            i += 1;
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

// Function-pointer table (mirrors C's `static operation_func operations[4]`).
type OperationFunc = extern "C" fn(c_int, c_int) -> c_int;

static OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

#[unsafe(no_mangle)]
pub extern "C" fn findrep(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
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
    let normalized_p4 = validate_and_normalize(param4);

    // Local char buffers (only used for side-effecting calls; not returned).
    let mut message: [c_char; 100] = [0; 100];
    let mut search_buffer: [c_char; 100] = [0; 100];

    unsafe {
        process_octal_string(message.as_mut_ptr(), 0o123);
    }

    // strcpy(search_buffer, "Function pointer example with static vars");
    let s = b"Function pointer example with static vars";
    for (i, &b) in s.iter().enumerate() {
        search_buffer[i] = b as c_char;
    }
    search_buffer[s.len()] = 0;
    let _ = &search_buffer;

    // memchr for 'p' within strlen(search_buffer) bytes.
    // Find offset in s.
    let mut found_offset: Option<usize> = None;
    for (i, &b) in s.iter().enumerate() {
        if b == b'p' {
            found_offset = Some(i);
            break;
        }
    }
    if let Some(off) = found_offset {
        result = result.wrapping_add(off as c_int);
    }

    if active_params >= mode_add {
        let selected_op = OPERATIONS[0];
        result = result.wrapping_add(selected_op(normalized_p1, normalized_p2));
    }

    if active_params >= mode_multiply {
        let selected_op = OPERATIONS[1];
        result = result.wrapping_add(selected_op(normalized_p3, normalized_p4));
    }

    let acc_now = unsafe { ACCUMULATOR };
    if acc_now > 0o150 {
        let selected_op = OPERATIONS[2];
        let subtract_result = selected_op(normalized_p1, normalized_p3);
        result = result.wrapping_add(subtract_result);
    }

    unsafe {
        find_and_replace_char(message.as_mut_ptr(), b'O' as c_int);
    }

    // strcpy(final_message, message); — no observable effect on result.
    let mut final_message: [c_char; 100] = [0; 100];
    {
        let mut i: usize = 0;
        loop {
            let c = message[i];
            final_message[i] = c;
            if c == 0 {
                break;
            }
            i += 1;
        }
    }
    let _ = final_message; // silence unused

    let (acc_now, mul_now) = unsafe { (ACCUMULATOR, MULTIPLIER) };
    let has_accumulator: c_int = if acc_now != 0 { 1 } else { 0 };
    let has_multiplier: c_int = if mul_now != 0 { 1 } else { 0 };
    let both_active: bool = has_accumulator != 0 && has_multiplier != 0;

    if both_active {
        result = result.wrapping_add(acc_now.wrapping_add(mul_now));
    }

    let mul_now2 = unsafe { MULTIPLIER };
    if mul_now2 > 0o100 {
        let selected_op = OPERATIONS[3];
        selected_op(mul_now2, 2);
    }

    let op_count = unsafe { OPERATION_COUNT };
    result = result.wrapping_add(op_count.wrapping_mul(0o10));

    let result_exists: c_int = if result != 0 { 1 } else { 0 };
    if result_exists == 0 {
        result = 0o777;
    }

    result
}
