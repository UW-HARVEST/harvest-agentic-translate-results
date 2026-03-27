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

type OperationFunc = extern "C" fn(c_int, c_int) -> c_int;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_octal_string(dest: *mut u8, octal_val: c_int) {
    let s = format!("Octal: 0{:o}, Decimal: {}", octal_val, octal_val);
    let bytes = s.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bytes.len());
        *dest.add(bytes.len()) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_and_replace_char(str_buf: *mut u8, search_char: c_int) {
    unsafe {
        let len = libc_strlen(str_buf);
        let slice = std::slice::from_raw_parts_mut(str_buf, len);
        let ch = search_char as u8;
        if let Some(pos) = slice.iter().position(|&b| b == ch) {
            slice[pos] = b'X';
        }
    }
}

unsafe fn libc_strlen(s: *const u8) -> usize {
    let mut len = 0;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    len
}

#[unsafe(no_mangle)]
pub extern "C" fn validate_and_normalize(value: c_int) -> c_int {
    let _is_nonzero = if value != 0 { 1 } else { 0 };
    let _is_zero = if value == 0 { 1 } else { 0 };

    let lower_threshold: c_int = 0o100;
    let upper_threshold: c_int = 0o777;

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

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0u8; 100];
    let mut search_buffer = [0u8; 100];

    process_octal_string_internal(&mut message, 0o123);

    let src = b"Function pointer example with static vars\0";
    search_buffer[..src.len()].copy_from_slice(src);

    // memchr for 'p' in search_buffer
    let len = search_buffer.iter().position(|&b| b == 0).unwrap_or(search_buffer.len());
    if let Some(pos) = search_buffer[..len].iter().position(|&b| b == b'p') {
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

    unsafe {
        if ACCUMULATOR > 0o150 {
            selected_op = OPERATIONS[2];
            let subtract_result = selected_op(normalized_p1, normalized_p3);
            result = result.wrapping_add(subtract_result);
        }
    }

    find_and_replace_char_internal(&mut message, b'O');

    let mut _final_message = [0u8; 100];
    let msg_len = message.iter().position(|&b| b == 0).unwrap_or(message.len());
    _final_message[..msg_len + 1].copy_from_slice(&message[..msg_len + 1]);

    unsafe {
        let has_accumulator = if ACCUMULATOR != 0 { 1 } else { 0 };
        let has_multiplier = if MULTIPLIER != 0 { 1 } else { 0 };
        let both_active = has_accumulator & has_multiplier;

        if both_active != 0 {
            result = result.wrapping_add(ACCUMULATOR.wrapping_add(MULTIPLIER));
        }

        if MULTIPLIER > 0o100 {
            selected_op = OPERATIONS[3];
            selected_op(MULTIPLIER, 2);
        }

        result = result.wrapping_add(OPERATION_COUNT.wrapping_mul(0o10));
    }

    let result_exists = if result != 0 { 1 } else { 0 };
    if result_exists == 0 {
        result = 0o777;
    }

    result
}

fn process_octal_string_internal(dest: &mut [u8], octal_val: c_int) {
    let s = format!("Octal: 0{:o}, Decimal: {}", octal_val, octal_val);
    let bytes = s.as_bytes();
    dest[..bytes.len()].copy_from_slice(bytes);
    dest[bytes.len()] = 0;
}

fn find_and_replace_char_internal(str_buf: &mut [u8], search_char: u8) {
    let len = str_buf.iter().position(|&b| b == 0).unwrap_or(str_buf.len());
    if let Some(pos) = str_buf[..len].iter().position(|&b| b == search_char) {
        str_buf[pos] = b'X';
    }
}
