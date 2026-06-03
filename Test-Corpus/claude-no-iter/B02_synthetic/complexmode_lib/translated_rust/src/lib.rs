// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output to the original C code.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

#[repr(C)]
struct Result {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

extern "C" {
    fn malloc(size: libc::size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: libc::size_t) -> *mut c_void;
    fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn snprintf(s: *mut c_char, n: libc::size_t, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let str_ptr = malloc(64 * std::mem::size_of::<c_char>()) as *mut c_char;
    if str_ptr.is_null() {
        return std::ptr::null_mut();
    }
    snprintf(
        str_ptr,
        64,
        b"Operation: %s, Value: %d\0".as_ptr() as *const c_char,
        op,
        val,
    );
    str_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    ((perms & required) == required) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        printf(b"Insufficient permissions for addition\n\0".as_ptr() as *const c_char);
        return 0;
    }
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_log(
    a: c_int,
    b: c_int,
    log_msg: *mut *mut c_char,
) -> c_int {
    *log_msg = create_result_string(
        b"multiply\0".as_ptr() as *const c_char,
        a.wrapping_mul(b),
    );
    if (*log_msg).is_null() {
        return 0;
    }
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        printf(b"Source pointer is NULL\n\0".as_ptr() as *const c_char);
        return -1;
    }

    let elem_size = std::mem::size_of::<c_int>();
    let total_size = (count as usize).wrapping_mul(elem_size);
    let dest = malloc(total_size) as *mut c_int;
    if dest.is_null() {
        printf(b"Memory allocation failed\n\0".as_ptr() as *const c_char);
        return -1;
    }

    memcpy(dest as *mut c_void, src as *const c_void, total_size);

    let mut sum: c_int = 0;
    for i in 0..count {
        sum = sum.wrapping_add(*dest.offset(i as isize));
    }

    free(dest as *mut c_void);
    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(
    op1: *const c_char,
    op2: *const c_char,
) -> c_int {
    if op1.is_null() || op2.is_null() {
        printf(b"One or both operation strings are NULL\n\0".as_ptr() as *const c_char);
        return -1;
    }
    strcmp(op1, op2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn complexmode(
    mode: c_int,
    value1: c_int,
    value2: c_int,
    value3: c_int,
) -> c_int {
    #[allow(unused_assignments)]
    let mut result: c_int = 0;
    let mut log_message: *mut c_char = std::ptr::null_mut();

    let permissions: c_int = 0o644; // rw-r--r--

    let res_tracker = malloc(std::mem::size_of::<Result>()) as *mut Result;
    if res_tracker.is_null() {
        printf(b"Failed to allocate result tracker\n\0".as_ptr() as *const c_char);
        return -1;
    }

    (*res_tracker).value = 0;
    (*res_tracker).permissions = permissions;
    strcpy(
        (*res_tracker).operation.as_mut_ptr(),
        b"none\0".as_ptr() as *const c_char,
    );

    match mode {
        1 => {
            strcpy(
                (*res_tracker).operation.as_mut_ptr(),
                b"addition\0".as_ptr() as *const c_char,
            );
            result = safe_add(value1, value2, permissions);
            (*res_tracker).value = result;

            printf(b"Mode 1: Addition\n\0".as_ptr() as *const c_char);
            printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
        }
        2 => {
            strcpy(
                (*res_tracker).operation.as_mut_ptr(),
                b"multiplication\0".as_ptr() as *const c_char,
            );
            result = multiply_with_log(value1, value2, &mut log_message);
            (*res_tracker).value = result;

            if log_message.is_null()
                || strcmp(log_message, b"\0".as_ptr() as *const c_char) == 0
            {
                printf(b"Log message creation failed\n\0".as_ptr() as *const c_char);
            } else {
                printf(
                    b"Mode 2: %s\n\0".as_ptr() as *const c_char,
                    log_message,
                );
                free(log_message as *mut c_void);
            }
        }
        3 => {
            strcpy(
                (*res_tracker).operation.as_mut_ptr(),
                b"array_sum\0".as_ptr() as *const c_char,
            );
            let mut values: [c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(values.as_mut_ptr(), 3);
            (*res_tracker).value = result;

            printf(b"Mode 3: Array Sum\n\0".as_ptr() as *const c_char);
            printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
        }
        4 => {
            strcpy(
                (*res_tracker).operation.as_mut_ptr(),
                b"complex\0".as_ptr() as *const c_char,
            );

            if check_permissions(permissions, 0o100) != 0 {
                result = value1.wrapping_mul(value2).wrapping_add(value3);
            } else {
                result = value1.wrapping_add(value2).wrapping_add(value3);
            }

            (*res_tracker).value = result;
            printf(b"Mode 4: Complex Calculation\n\0".as_ptr() as *const c_char);
            printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
        }
        _ => {
            printf(b"Invalid mode\n\0".as_ptr() as *const c_char);
            result = -1;
        }
    }

    if strcmp(
        (*res_tracker).operation.as_ptr(),
        b"none\0".as_ptr() as *const c_char,
    ) != 0
    {
        printf(
            b"Operation performed: %s\n\0".as_ptr() as *const c_char,
            (*res_tracker).operation.as_ptr(),
        );
    }

    free(res_tracker as *mut c_void);

    result
}
