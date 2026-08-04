// Translated from c_src/src/lib.c — preserves original behavior including
// static (global) state across calls within a single process.

use std::cell::Cell;

thread_local! {
    static ACCUMULATOR: Cell<i32> = const { Cell::new(0) };
    static MULTIPLIER: Cell<i32> = const { Cell::new(1) };
    static OPERATION_COUNT: Cell<i32> = const { Cell::new(0) };
}

fn accumulator_get() -> i32 {
    ACCUMULATOR.with(|c| c.get())
}
fn accumulator_set(v: i32) {
    ACCUMULATOR.with(|c| c.set(v));
}
fn multiplier_get() -> i32 {
    MULTIPLIER.with(|c| c.get())
}
fn multiplier_set(v: i32) {
    MULTIPLIER.with(|c| c.set(v));
}
fn operation_count_get() -> i32 {
    OPERATION_COUNT.with(|c| c.get())
}
fn operation_count_inc() {
    OPERATION_COUNT.with(|c| c.set(c.get().wrapping_add(1)));
}

type OperationFunc = fn(i32, i32) -> i32;

fn add_to_accumulator(a: i32, b: i32) -> i32 {
    let v = accumulator_get().wrapping_add(a.wrapping_add(b));
    accumulator_set(v);
    operation_count_inc();
    accumulator_get()
}

fn multiply_with_multiplier(a: i32, b: i32) -> i32 {
    let v = multiplier_get().wrapping_mul(a.wrapping_mul(b));
    multiplier_set(v);
    operation_count_inc();
    multiplier_get()
}

fn subtract_from_accumulator(a: i32, b: i32) -> i32 {
    let v = accumulator_get().wrapping_sub(a.wrapping_sub(b));
    accumulator_set(v);
    operation_count_inc();
    accumulator_get()
}

fn divide_multiplier(_a: i32, b: i32) -> i32 {
    if b != 0 {
        let v = multiplier_get() / b;
        multiplier_set(v);
    }
    operation_count_inc();
    multiplier_get()
}

/// Mimics C `process_octal_string(dest, octal_val)` — writes a formatted
/// string into `dest` using the same format as
/// `sprintf("Octal: 0%o, Decimal: %d", octal_val, octal_val)`.
fn process_octal_string(dest: &mut String, octal_val: i32) {
    dest.clear();
    // C `%o` formats as unsigned octal. For non-negative values this matches
    // Rust's `{:o}` on a `u32` cast.
    let unsigned = octal_val as u32;
    *dest = format!("Octal: 0{:o}, Decimal: {}", unsigned, octal_val);
}

/// Mimics C `find_and_replace_char(str, search_char)` — replaces the first
/// occurrence of `search_char` (interpreted as a byte) with 'X'.
fn find_and_replace_char(s: &mut Vec<u8>, search_char: u8) {
    if let Some(pos) = s.iter().position(|&b| b == search_char) {
        s[pos] = b'X';
    }
}

fn validate_and_normalize(value: i32) -> i32 {
    let is_nonzero = (value != 0) as i32;
    let _is_zero = (value == 0) as i32;

    let lower_threshold: i32 = 0o100; // 64
    let upper_threshold: i32 = 0o777; // 511

    if is_nonzero != 0 && value > 0 {
        if value < lower_threshold {
            return lower_threshold;
        } else if value > upper_threshold {
            return upper_threshold;
        }
    }

    value
}

const OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

pub fn findrep(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result: i32 = 0;

    let p1_valid = (param1 != 0) as i32;
    let p2_valid = (param2 != 0) as i32;
    let p3_valid = (param3 != 0) as i32;
    let p4_valid = (param4 != 0) as i32;

    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add: i32 = 0o1;
    let mode_multiply: i32 = 0o2;
    let _mode_subtract: i32 = 0o3;
    let _mode_divide: i32 = 0o4;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = String::with_capacity(100);
    let mut search_buffer: Vec<u8> = Vec::with_capacity(100);

    process_octal_string(&mut message, 0o123);
    search_buffer.extend_from_slice(b"Function pointer example with static vars");

    // memchr for 'p' across the C string length (strlen) — equivalent to
    // searching the entire byte buffer up to (but not including) the NUL.
    if let Some(pos) = search_buffer.iter().position(|&b| b == b'p') {
        result = result.wrapping_add(pos as i32);
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

    if accumulator_get() > 0o150 {
        selected_op = OPERATIONS[2];
        let subtract_result = selected_op(normalized_p1, normalized_p3);
        result = result.wrapping_add(subtract_result);
    }

    // Convert message into a mutable byte buffer for the in-place replacement.
    let mut message_bytes: Vec<u8> = message.into_bytes();
    find_and_replace_char(&mut message_bytes, b'O');

    // Mirror the unused `final_message` strcpy in the original C.
    let _final_message: Vec<u8> = message_bytes.clone();

    let has_accumulator = (accumulator_get() != 0) as i32;
    let has_multiplier = (multiplier_get() != 0) as i32;
    let both_active = (has_accumulator != 0) && (has_multiplier != 0);

    if both_active {
        result = result.wrapping_add(accumulator_get().wrapping_add(multiplier_get()));
    }

    if multiplier_get() > 0o100 {
        selected_op = OPERATIONS[3];
        let _ = selected_op(multiplier_get(), 2);
    }

    result = result.wrapping_add(operation_count_get().wrapping_mul(0o10));

    let result_exists = (result != 0) as i32;
    if result_exists == 0 {
        result = 0o777;
    }

    result
}
