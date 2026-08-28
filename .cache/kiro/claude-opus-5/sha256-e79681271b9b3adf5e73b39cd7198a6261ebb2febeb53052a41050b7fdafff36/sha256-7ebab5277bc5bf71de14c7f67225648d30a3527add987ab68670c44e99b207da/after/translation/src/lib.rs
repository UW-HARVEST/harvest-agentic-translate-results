// Rust translation of c_src/src/lib.c
//
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

use std::ffi::{c_char, c_int, c_void};

// The C code relies on the platform allocator (malloc/realloc/free) and on
// stdio's `printf` for output. Both are used directly here so that the
// allocation ownership semantics and the stdout buffering behaviour of the
// original translation unit are preserved exactly.
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[repr(C)]
pub struct StringBuffer {
    data: *mut c_char,
    capacity: c_int,
    length: c_int,
}

/// `strlen` equivalent, returning the length as a C `int` exactly as the
/// original code does (the C source assigns `strlen`'s `size_t` result to an
/// `int`, so the truncation is part of the observable behaviour).
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    unsafe {
        while *s.add(n) != 0 {
            n += 1;
        }
    }
    n
}

/// Compare a NUL-terminated C string against a Rust byte literal (without the
/// terminator), mirroring `strcmp(operation, "...") == 0`.
unsafe fn c_str_eq(s: *const c_char, expected: &[u8]) -> bool {
    unsafe {
        let mut i = 0usize;
        while i < expected.len() {
            let b = *s.add(i) as u8;
            if b != expected[i] {
                return false;
            }
            i += 1;
        }
        *s.add(expected.len()) == 0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    unsafe {
        let buffer = malloc(size_of::<StringBuffer>()) as *mut StringBuffer;
        if buffer.is_null() {
            return std::ptr::null_mut();
        }

        // `malloc(initial_capacity)` converts the `int` to `size_t`, which
        // sign-extends on LP64; the `as usize` cast reproduces that.
        let data = malloc(initial_capacity as usize) as *mut c_char;
        if data.is_null() {
            free(buffer as *mut c_void);
            return std::ptr::null_mut();
        }

        (*buffer).data = data;
        (*buffer).capacity = initial_capacity;
        (*buffer).length = 0;
        // Unconditional write, exactly as in the C (no capacity check).
        *(*buffer).data = 0;

        buffer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn append_to_buffer(buffer: *mut StringBuffer, str: *const c_char) -> c_int {
    unsafe {
        let str_len_usize = c_strlen(str);
        let str_len: c_int = str_len_usize as c_int;
        let required_capacity: c_int = (*buffer)
            .length
            .wrapping_add(str_len)
            .wrapping_add(1);

        if required_capacity > (*buffer).capacity {
            let new_capacity: c_int = required_capacity.wrapping_mul(2);
            let new_data =
                realloc((*buffer).data as *mut c_void, new_capacity as usize) as *mut c_char;

            if new_data.is_null() {
                return -1;
            }

            (*buffer).data = new_data;
            (*buffer).capacity = new_capacity;
        }

        // strcpy(buffer->data + buffer->length, str)
        let dst = (*buffer).data.offset((*buffer).length as isize);
        std::ptr::copy_nonoverlapping(str, dst, str_len_usize + 1);
        (*buffer).length = (*buffer).length.wrapping_add(str_len);

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    unsafe {
        if !buffer.is_null() {
            if !(*buffer).data.is_null() {
                free((*buffer).data as *mut c_void);
            }
            free(buffer as *mut c_void);
        }
    }
}

const OP_ADD: &[u8] = b"add\0";
const OP_SUBTRACT: &[u8] = b"subtract\0";
const OP_MULTIPLY: &[u8] = b"multiply\0";
const OP_DIVIDE: &[u8] = b"divide\0";
const OP_UNKNOWN: &[u8] = b"unknown\0";

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_name(op_code: c_int) -> *const c_char {
    let s: &[u8] = match op_code {
        0 => OP_ADD,
        1 => OP_SUBTRACT,
        2 => OP_MULTIPLY,
        3 => OP_DIVIDE,
        _ => OP_UNKNOWN,
    };
    s.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(a: c_int, b: c_int, operation: *const c_char) -> c_int {
    unsafe {
        // `wrapping_*` keeps the two's-complement result of the C code's
        // signed overflow instead of panicking.
        if c_str_eq(operation, b"add") {
            a.wrapping_add(b)
        } else if c_str_eq(operation, b"subtract") {
            a.wrapping_sub(b)
        } else if c_str_eq(operation, b"multiply") {
            a.wrapping_mul(b)
        } else if c_str_eq(operation, b"divide") {
            if b != 0 {
                a.wrapping_div(b)
            } else {
                0
            }
        } else {
            0
        }
    }
}

/// Renders into a NUL-terminated scratch buffer, standing in for
/// `sprintf(temp, ...)` over `char temp[64]`.
fn sprintf_temp(temp: &mut Vec<u8>, formatted: &str) {
    temp.clear();
    temp.extend_from_slice(formatted.as_bytes());
    temp.push(0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffapp(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        let log_buffer = create_buffer(32);
        let mut result: c_int = 0;
        let mut temp: Vec<u8> = Vec::with_capacity(64);

        // No NULL check in the C source; dereferencing a failed allocation is
        // reproduced as-is.
        (*log_buffer).length = 0;

        sprintf_temp(
            &mut temp,
            &format!("Starting computation with {} parameters\n", 4),
        );
        append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

        // C's `%` truncates toward zero, so a negative param yields a negative
        // op code and therefore "unknown".
        let op1 = get_operation_name(param1.wrapping_rem(4));
        sprintf_temp(
            &mut temp,
            &format!(
                "Operation 1: {}({}, {})\n",
                cstr_display(op1),
                param1,
                param2
            ),
        );
        append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

        let intermediate1 = perform_operation(param1, param2, op1);
        result = result.wrapping_add(intermediate1);

        let op2 = get_operation_name(param3.wrapping_rem(4));
        sprintf_temp(
            &mut temp,
            &format!(
                "Operation 2: {}({}, {})\n",
                cstr_display(op2),
                param3,
                param4
            ),
        );
        append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

        let intermediate2 = perform_operation(param3, param4, op2);
        result = result.wrapping_add(intermediate2);

        let op3 = OP_MULTIPLY.as_ptr() as *const c_char;
        sprintf_temp(
            &mut temp,
            &format!(
                "Operation 3: {}({}, {})\n",
                cstr_display(op3),
                intermediate1,
                intermediate2
            ),
        );
        append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

        let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

        if intermediate3 != 0 {
            result = result.wrapping_div(intermediate3);
        } else {
            result = param1
                .wrapping_add(param2)
                .wrapping_add(param3)
                .wrapping_add(param4);
        }

        sprintf_temp(&mut temp, &format!("Final result: {}\n", result));
        append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

        printf(
            b"Computation Log:\n%s\n\0".as_ptr() as *const c_char,
            (*log_buffer).data,
        );

        destroy_buffer(log_buffer);

        result
    }
}

/// Borrow a NUL-terminated C string as `&str` for formatting. All call sites
/// pass ASCII literals from `get_operation_name`.
unsafe fn cstr_display(s: *const c_char) -> &'static str {
    unsafe {
        let len = c_strlen(s);
        let bytes = std::slice::from_raw_parts(s as *const u8, len);
        std::str::from_utf8_unchecked(bytes)
    }
}
