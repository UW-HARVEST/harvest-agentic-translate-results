// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to match the original C library byte-for-byte.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::CStr;

#[repr(C)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    let buffer = libc::malloc(std::mem::size_of::<StringBuffer>()) as *mut StringBuffer;
    if buffer.is_null() {
        return std::ptr::null_mut();
    }

    let data = libc::malloc(initial_capacity as usize) as *mut c_char;
    if data.is_null() {
        libc::free(buffer as *mut libc::c_void);
        return std::ptr::null_mut();
    }

    (*buffer).data = data;
    (*buffer).capacity = initial_capacity;
    (*buffer).length = 0;
    *(*buffer).data = 0;

    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn append_to_buffer(buffer: *mut StringBuffer, s: *const c_char) -> c_int {
    let str_len = libc::strlen(s) as c_int;
    let required_capacity = (*buffer).length + str_len + 1;

    if required_capacity > (*buffer).capacity {
        let new_capacity = required_capacity * 2;
        let new_data = libc::realloc(
            (*buffer).data as *mut libc::c_void,
            new_capacity as usize,
        ) as *mut c_char;

        if new_data.is_null() {
            return -1;
        }

        (*buffer).data = new_data;
        (*buffer).capacity = new_capacity;
    }

    libc::strcpy((*buffer).data.offset((*buffer).length as isize), s);
    (*buffer).length += str_len;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    if !buffer.is_null() {
        if !(*buffer).data.is_null() {
            libc::free((*buffer).data as *mut libc::c_void);
        }
        libc::free(buffer as *mut libc::c_void);
    }
}

// String literals for operation names. We use static C strings so that
// callers see the same kinds of pointers as the C version (string
// literals are NUL-terminated and have static storage duration).
static OP_ADD: &[u8] = b"add\0";
static OP_SUB: &[u8] = b"subtract\0";
static OP_MUL: &[u8] = b"multiply\0";
static OP_DIV: &[u8] = b"divide\0";
static OP_UNK: &[u8] = b"unknown\0";

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_name(op_code: c_int) -> *const c_char {
    let bytes: &[u8] = match op_code {
        0 => OP_ADD,
        1 => OP_SUB,
        2 => OP_MUL,
        3 => OP_DIV,
        _ => OP_UNK,
    };
    bytes.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(a: c_int, b: c_int, operation: *const c_char) -> c_int {
    let op = CStr::from_ptr(operation).to_bytes();
    if op == b"add" {
        a.wrapping_add(b)
    } else if op == b"subtract" {
        a.wrapping_sub(b)
    } else if op == b"multiply" {
        a.wrapping_mul(b)
    } else if op == b"divide" {
        if b != 0 {
            // C `/` for ints: truncated division. wrapping_div matches C
            // behavior on the INT_MIN / -1 case (which is UB in C).
            a.wrapping_div(b)
        } else {
            0
        }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffapp(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let log_buffer = create_buffer(32);
    let mut result: c_int = 0;
    let mut temp: [c_char; 64] = [0; 64];

    (*log_buffer).length = 0;

    libc::sprintf(
        temp.as_mut_ptr(),
        b"Starting computation with %d parameters\n\0".as_ptr() as *const c_char,
        4 as c_int,
    );
    append_to_buffer(log_buffer, temp.as_ptr());

    let op1 = get_operation_name(param1.wrapping_rem(4));
    libc::sprintf(
        temp.as_mut_ptr(),
        b"Operation 1: %s(%d, %d)\n\0".as_ptr() as *const c_char,
        op1,
        param1,
        param2,
    );
    append_to_buffer(log_buffer, temp.as_ptr());

    let intermediate1 = perform_operation(param1, param2, op1);
    result = result.wrapping_add(intermediate1);

    let op2 = get_operation_name(param3.wrapping_rem(4));
    libc::sprintf(
        temp.as_mut_ptr(),
        b"Operation 2: %s(%d, %d)\n\0".as_ptr() as *const c_char,
        op2,
        param3,
        param4,
    );
    append_to_buffer(log_buffer, temp.as_ptr());

    let intermediate2 = perform_operation(param3, param4, op2);
    result = result.wrapping_add(intermediate2);

    let op3 = b"multiply\0".as_ptr() as *const c_char;
    libc::sprintf(
        temp.as_mut_ptr(),
        b"Operation 3: %s(%d, %d)\n\0".as_ptr() as *const c_char,
        op3,
        intermediate1,
        intermediate2,
    );
    append_to_buffer(log_buffer, temp.as_ptr());

    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result = result.wrapping_div(intermediate3);
    } else {
        result = param1
            .wrapping_add(param2)
            .wrapping_add(param3)
            .wrapping_add(param4);
    }

    libc::sprintf(
        temp.as_mut_ptr(),
        b"Final result: %d\n\0".as_ptr() as *const c_char,
        result,
    );
    append_to_buffer(log_buffer, temp.as_ptr());

    libc::printf(
        b"Computation Log:\n%s\n\0".as_ptr() as *const c_char,
        (*log_buffer).data,
    );

    destroy_buffer(log_buffer);

    result
}
