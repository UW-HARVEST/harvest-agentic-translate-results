// Rust translation of c_src/src/lib.c
// Preserves byte-identical behavior of the original C code.

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Module-level mutable state, mirroring the C `static` globals.
// The original C code is not thread-safe; we mirror that exactly using
// `static mut`.
// ---------------------------------------------------------------------------
static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

// ---------------------------------------------------------------------------
// Function-pointer alias mirroring `typedef int (*operation_func)(int, int);`
// ---------------------------------------------------------------------------
type OperationFunc = fn(c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Operation helpers (these have side effects on the static globals).
// ---------------------------------------------------------------------------
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
            MULTIPLIER /= b;
        }
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

// ---------------------------------------------------------------------------
// String helpers from the C source. They operate on local buffers within
// `findrep` only — we mirror their behavior even though they don't directly
// influence the return value (the C versions also touch only local buffers).
// ---------------------------------------------------------------------------
fn process_octal_string(dest: &mut [u8], octal_val: c_int) {
    // Mirrors: sprintf(buffer, "Octal: 0%o, Decimal: %d", octal_val, octal_val);
    // C `%o` prints unsigned octal of the int (reinterpreted as unsigned).
    let unsigned_val = octal_val as u32;
    let formatted = format!("Octal: 0{:o}, Decimal: {}", unsigned_val, octal_val);
    let bytes = formatted.as_bytes();
    let n = bytes.len();
    // Replicate strcpy: copy bytes plus the terminating NUL.
    dest[..n].copy_from_slice(bytes);
    dest[n] = 0;
}

fn find_and_replace_char(buf: &mut [u8], search_char: c_int) {
    // Mimics: char* found = memchr(str, search_char, strlen(str)); if (found) *found = 'X';
    // Determine the C-style string length (up to NUL terminator).
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    // memchr matches the byte equal to (unsigned char)search_char.
    let needle = (search_char as u32 & 0xFF) as u8;
    if let Some(idx) = buf[..len].iter().position(|&c| c == needle) {
        buf[idx] = b'X';
    }
}

fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero: c_int = (value != 0) as c_int;
    let _is_zero: c_int = (value == 0) as c_int;

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

// ---------------------------------------------------------------------------
// Function-pointer table mirroring `static operation_func operations[4]`.
// ---------------------------------------------------------------------------
const OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

// ---------------------------------------------------------------------------
// Public entry point exposed via the C ABI.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn findrep(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let p1_valid: c_int = (param1 != 0) as c_int;
    let p2_valid: c_int = (param2 != 0) as c_int;
    let p3_valid: c_int = (param3 != 0) as c_int;
    let p4_valid: c_int = (param4 != 0) as c_int;

    let active_params: c_int = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add: c_int = 0o1;
    let mode_multiply: c_int = 0o2;
    let _mode_subtract: c_int = 0o3;
    let _mode_divide: c_int = 0o4;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message: [u8; 100] = [0; 100];
    let mut search_buffer: [u8; 100] = [0; 100];

    process_octal_string(&mut message, 0o123);

    // strcpy(search_buffer, "Function pointer example with static vars");
    let src = b"Function pointer example with static vars";
    search_buffer[..src.len()].copy_from_slice(src);
    search_buffer[src.len()] = 0;

    // memchr(search_buffer, 'p', strlen(search_buffer))
    let sb_len = search_buffer
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(search_buffer.len());
    if let Some(idx) = search_buffer[..sb_len].iter().position(|&c| c == b'p') {
        result = result.wrapping_add(idx as c_int);
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

    find_and_replace_char(&mut message, 'O' as c_int);

    // char final_message[100]; strcpy(final_message, message);
    let mut final_message: [u8; 100] = [0; 100];
    let msg_len = message
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(message.len());
    final_message[..msg_len].copy_from_slice(&message[..msg_len]);
    if msg_len < final_message.len() {
        final_message[msg_len] = 0;
    }
    // Touch the buffer so the optimizer cannot eliminate the copy
    // (mirrors the unused C buffer's presence; not strictly required but
    // keeps semantics close to source).
    let _ = final_message[0];

    let (acc_now, mult_now) = unsafe { (ACCUMULATOR, MULTIPLIER) };
    let has_accumulator: c_int = (acc_now != 0) as c_int;
    let has_multiplier: c_int = (mult_now != 0) as c_int;
    let both_active = has_accumulator != 0 && has_multiplier != 0;

    if both_active {
        result = result.wrapping_add(acc_now.wrapping_add(mult_now));
    }

    let mult_for_check = unsafe { MULTIPLIER };
    if mult_for_check > 0o100 {
        selected_op = OPERATIONS[3];
        // The C code passes the *current* MULTIPLIER value as `a`, but
        // `divide_multiplier` ignores `a`. Match call exactly anyway.
        selected_op(mult_for_check, 2);
    }

    let op_count_now = unsafe { OPERATION_COUNT };
    result = result.wrapping_add(op_count_now.wrapping_mul(0o10));

    let result_exists: c_int = (result != 0) as c_int;
    if result_exists == 0 {
        result = 0o777;
    }

    result
}
