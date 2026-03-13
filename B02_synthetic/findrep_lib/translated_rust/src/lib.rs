// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::os::raw::c_int;

static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

type OperationFunc = fn(c_int, c_int) -> c_int;

fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR += a + b;
        OPERATION_COUNT += 1;
        ACCUMULATOR
    }
}

fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    unsafe {
        MULTIPLIER *= a * b;
        OPERATION_COUNT += 1;
        MULTIPLIER
    }
}

fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR -= a - b;
        OPERATION_COUNT += 1;
        ACCUMULATOR
    }
}

fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    unsafe {
        if b != 0 {
            MULTIPLIER /= b;
        }
        OPERATION_COUNT += 1;
        MULTIPLIER
    }
}

fn process_octal_string(dest: &mut [u8], octal_val: c_int) {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(&mut dest[..]);
    let _ = write!(cursor, "Octal: 0{:o}, Decimal: {}", octal_val, octal_val);
    let pos = cursor.position() as usize;
    dest[pos] = 0; // null terminator
}

fn find_and_replace_char(str_buf: &mut [u8], search_char: u8) {
    // strlen equivalent: find first null byte
    let len = str_buf.iter().position(|&b| b == 0).unwrap_or(str_buf.len());
    // memchr equivalent
    if let Some(pos) = str_buf[..len].iter().position(|&b| b == search_char) {
        str_buf[pos] = b'X';
    }
}

fn validate_and_normalize(value: c_int) -> c_int {
    let _is_nonzero = if value != 0 { 1 } else { 0 };
    let _is_zero = if value == 0 { 1 } else { 0 };

    let lower_threshold: c_int = 0o100; // 64
    let upper_threshold: c_int = 0o777; // 511

    if _is_nonzero != 0 && value > 0 {
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

#[unsafe(no_mangle)]
pub extern "C" fn findrep(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let p1_valid = if param1 != 0 { 1 } else { 0 };
    let p2_valid = if param2 != 0 { 1 } else { 0 };
    let p3_valid = if param3 != 0 { 1 } else { 0 };
    let p4_valid = if param4 != 0 { 1 } else { 0 };

    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add: c_int = 0o1;
    let mode_multiply: c_int = 0o2;
    let _mode_subtract: c_int = 0o3;
    let _mode_divide: c_int = 0o4;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0u8; 100];
    let mut search_buffer = [0u8; 100];

    process_octal_string(&mut message, 0o123); // 83 decimal

    // strcpy equivalent
    let src = b"Function pointer example with static vars\0";
    search_buffer[..src.len()].copy_from_slice(src);

    // memchr for 'p' in search_buffer
    let search_len = search_buffer.iter().position(|&b| b == 0).unwrap_or(search_buffer.len());
    if let Some(pos) = search_buffer[..search_len].iter().position(|&b| b == b'p') {
        result += pos as c_int;
    }

    let mut selected_op: OperationFunc;

    if active_params >= mode_add {
        selected_op = OPERATIONS[0];
        result += selected_op(normalized_p1, normalized_p2);
    }

    if active_params >= mode_multiply {
        selected_op = OPERATIONS[1];
        result += selected_op(normalized_p3, normalized_p4);
    }

    unsafe {
        if ACCUMULATOR > 0o150 { // 104 decimal
            selected_op = OPERATIONS[2];
            let subtract_result = selected_op(normalized_p1, normalized_p3);
            result += subtract_result;
        }
    }

    find_and_replace_char(&mut message, b'O');

    let mut _final_message = [0u8; 100];
    // strcpy equivalent
    let msg_len = message.iter().position(|&b| b == 0).unwrap_or(message.len());
    _final_message[..msg_len + 1].copy_from_slice(&message[..msg_len + 1]);

    unsafe {
        let has_accumulator = if ACCUMULATOR != 0 { 1 } else { 0 };
        let has_multiplier = if MULTIPLIER != 0 { 1 } else { 0 };
        let both_active = has_accumulator != 0 && has_multiplier != 0;

        if both_active {
            result += ACCUMULATOR + MULTIPLIER;
        }

        if MULTIPLIER > 0o100 { // 64 decimal
            selected_op = OPERATIONS[3];
            selected_op(MULTIPLIER, 2);
        }

        result += OPERATION_COUNT * 0o10; // 8 decimal

        let result_exists = if result != 0 { 1 } else { 0 };
        if result_exists == 0 {
            result = 0o777; // 511 decimal
        }
    }

    result
}
