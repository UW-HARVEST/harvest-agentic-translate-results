// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::sync::atomic::{AtomicI32, Ordering};

type OperationFunc = fn(i32, i32) -> i32;

static ACCUMULATOR: AtomicI32 = AtomicI32::new(0);
static MULTIPLIER: AtomicI32 = AtomicI32::new(1);
static OPERATION_COUNT: AtomicI32 = AtomicI32::new(0);

fn add_to_accumulator(a: i32, b: i32) -> i32 {
    let new_val = ACCUMULATOR.load(Ordering::SeqCst).wrapping_add(a.wrapping_add(b));
    ACCUMULATOR.store(new_val, Ordering::SeqCst);
    OPERATION_COUNT.fetch_add(1, Ordering::SeqCst);
    new_val
}

fn multiply_with_multiplier(a: i32, b: i32) -> i32 {
    let new_val = MULTIPLIER
        .load(Ordering::SeqCst)
        .wrapping_mul(a.wrapping_mul(b));
    MULTIPLIER.store(new_val, Ordering::SeqCst);
    OPERATION_COUNT.fetch_add(1, Ordering::SeqCst);
    new_val
}

fn subtract_from_accumulator(a: i32, b: i32) -> i32 {
    let new_val = ACCUMULATOR
        .load(Ordering::SeqCst)
        .wrapping_sub(a.wrapping_sub(b));
    ACCUMULATOR.store(new_val, Ordering::SeqCst);
    OPERATION_COUNT.fetch_add(1, Ordering::SeqCst);
    new_val
}

fn divide_multiplier(_a: i32, b: i32) -> i32 {
    if b != 0 {
        let cur = MULTIPLIER.load(Ordering::SeqCst);
        // Match C integer division (truncation toward zero).
        let new_val = cur.wrapping_div(b);
        MULTIPLIER.store(new_val, Ordering::SeqCst);
    }
    OPERATION_COUNT.fetch_add(1, Ordering::SeqCst);
    MULTIPLIER.load(Ordering::SeqCst)
}

fn process_octal_string(dest: &mut Vec<u8>, octal_val: i32) {
    // Mimic sprintf(buffer, "Octal: 0%o, Decimal: %d", octal_val, octal_val);
    let s = format!("Octal: 0{:o}, Decimal: {}", octal_val, octal_val);
    dest.clear();
    dest.extend_from_slice(s.as_bytes());
    dest.push(0); // NUL terminator like C strcpy
}

fn find_and_replace_char(s: &mut [u8], search_char: u8) {
    // Replicate memchr over the C-string length (up to NUL).
    let len = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    if let Some(idx) = s[..len].iter().position(|&c| c == search_char) {
        s[idx] = b'X';
    }
}

fn validate_and_normalize(value: i32) -> i32 {
    let is_nonzero = value != 0;

    let lower_threshold: i32 = 0o100; // 64
    let upper_threshold: i32 = 0o777; // 511

    if is_nonzero && value > 0 {
        if value < lower_threshold {
            return lower_threshold;
        } else if value > upper_threshold {
            return upper_threshold;
        }
    }

    value
}

static OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

#[no_mangle]
pub extern "C" fn findrep(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
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

    let mut message: Vec<u8> = vec![0u8; 100];
    let mut search_buffer: Vec<u8> = vec![0u8; 100];

    process_octal_string(&mut message, 0o123);

    // strcpy(search_buffer, "Function pointer example with static vars");
    let src = b"Function pointer example with static vars";
    search_buffer.clear();
    search_buffer.extend_from_slice(src);
    search_buffer.push(0);

    // Find 'p' in search_buffer and add its index to result.
    let sb_len = search_buffer
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(search_buffer.len());
    if let Some(idx) = search_buffer[..sb_len].iter().position(|&c| c == b'p') {
        result = result.wrapping_add(idx as i32);
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

    if ACCUMULATOR.load(Ordering::SeqCst) > 0o150 {
        selected_op = OPERATIONS[2];
        let subtract_result = selected_op(normalized_p1, normalized_p3);
        result = result.wrapping_add(subtract_result);
    }

    find_and_replace_char(&mut message, b'O');

    // char final_message[100]; strcpy(final_message, message);
    let mut final_message: Vec<u8> = vec![0u8; 100];
    let msg_len = message.iter().position(|&c| c == 0).unwrap_or(message.len());
    final_message[..msg_len].copy_from_slice(&message[..msg_len]);
    final_message[msg_len] = 0;
    let _ = final_message; // suppress unused warning

    let has_accumulator = ACCUMULATOR.load(Ordering::SeqCst) != 0;
    let has_multiplier = MULTIPLIER.load(Ordering::SeqCst) != 0;
    let both_active = has_accumulator && has_multiplier;

    if both_active {
        result = result.wrapping_add(
            ACCUMULATOR
                .load(Ordering::SeqCst)
                .wrapping_add(MULTIPLIER.load(Ordering::SeqCst)),
        );
    }

    if MULTIPLIER.load(Ordering::SeqCst) > 0o100 {
        selected_op = OPERATIONS[3];
        let _ = selected_op(MULTIPLIER.load(Ordering::SeqCst), 2);
    }

    result = result.wrapping_add(OPERATION_COUNT.load(Ordering::SeqCst).wrapping_mul(0o10));

    let result_exists = result != 0;
    if !result_exists {
        result = 0o777;
    }

    result
}
