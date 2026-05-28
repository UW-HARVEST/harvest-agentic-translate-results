// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior preserved byte-for-byte.

use std::ffi::c_char;
use std::ffi::c_int;
use std::os::raw::c_void;

#[repr(C)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    unsafe {
        let buffer = malloc(std::mem::size_of::<StringBuffer>()) as *mut StringBuffer;
        if buffer.is_null() {
            return std::ptr::null_mut();
        }

        let data = malloc(initial_capacity as usize) as *mut c_char;
        if data.is_null() {
            free(buffer as *mut c_void);
            return std::ptr::null_mut();
        }

        (*buffer).data = data;
        (*buffer).capacity = initial_capacity;
        (*buffer).length = 0;
        *(*buffer).data = 0;

        buffer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn append_to_buffer(buffer: *mut StringBuffer, s: *const c_char) -> c_int {
    unsafe {
        let str_len = strlen(s) as c_int;
        let required_capacity = (*buffer).length + str_len + 1;

        if required_capacity > (*buffer).capacity {
            let new_capacity = required_capacity * 2;
            let new_data =
                realloc((*buffer).data as *mut c_void, new_capacity as usize) as *mut c_char;

            if new_data.is_null() {
                return -1;
            }

            (*buffer).data = new_data;
            (*buffer).capacity = new_capacity;
        }

        strcpy((*buffer).data.add((*buffer).length as usize), s);
        (*buffer).length += str_len;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_operation_name(op_code: c_int) -> *const c_char {
    match op_code {
        0 => b"add\0".as_ptr() as *const c_char,
        1 => b"subtract\0".as_ptr() as *const c_char,
        2 => b"multiply\0".as_ptr() as *const c_char,
        3 => b"divide\0".as_ptr() as *const c_char,
        _ => b"unknown\0".as_ptr() as *const c_char,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(a: c_int, b: c_int, operation: *const c_char) -> c_int {
    unsafe {
        if strcmp(operation, b"add\0".as_ptr() as *const c_char) == 0 {
            return a.wrapping_add(b);
        } else if strcmp(operation, b"subtract\0".as_ptr() as *const c_char) == 0 {
            return a.wrapping_sub(b);
        } else if strcmp(operation, b"multiply\0".as_ptr() as *const c_char) == 0 {
            return a.wrapping_mul(b);
        } else if strcmp(operation, b"divide\0".as_ptr() as *const c_char) == 0 {
            if b != 0 {
                return a / b;
            }
            return 0;
        }
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
    unsafe {
        let log_buffer = create_buffer(32);
        let mut result: c_int = 0;
        let mut temp: [c_char; 64] = [0; 64];

        (*log_buffer).length = 0;

        sprintf(
            temp.as_mut_ptr(),
            b"Starting computation with %d parameters\n\0".as_ptr() as *const c_char,
            4 as c_int,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        let op1 = get_operation_name(param1 % 4);
        sprintf(
            temp.as_mut_ptr(),
            b"Operation 1: %s(%d, %d)\n\0".as_ptr() as *const c_char,
            op1,
            param1,
            param2,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        let intermediate1 = perform_operation(param1, param2, op1);
        result = result.wrapping_add(intermediate1);

        let op2 = get_operation_name(param3 % 4);
        sprintf(
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
        sprintf(
            temp.as_mut_ptr(),
            b"Operation 3: %s(%d, %d)\n\0".as_ptr() as *const c_char,
            op3,
            intermediate1,
            intermediate2,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

        if intermediate3 != 0 {
            result = result / intermediate3;
        } else {
            result = param1
                .wrapping_add(param2)
                .wrapping_add(param3)
                .wrapping_add(param4);
        }

        sprintf(
            temp.as_mut_ptr(),
            b"Final result: %d\n\0".as_ptr() as *const c_char,
            result,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        printf(
            b"Computation Log:\n%s\n\0".as_ptr() as *const c_char,
            (*log_buffer).data,
        );

        destroy_buffer(log_buffer);

        result
    }
}
