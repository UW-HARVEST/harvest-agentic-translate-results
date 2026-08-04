// SPDX-License-Identifier: MIT
// Rust translation of c_src/src/lib.c
// Preserves byte-identical behavior including stdout output through libc::printf.

use std::ffi::c_char;
use std::ffi::c_int;

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

/// Equivalent to: char* create_result_string(const char* op, int val)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    // char* str = (char*)malloc(64 * sizeof(char));
    let str_ptr = libc::malloc(64 * std::mem::size_of::<c_char>()) as *mut c_char;
    if str_ptr.is_null() {
        return std::ptr::null_mut();
    }
    // snprintf(str, 64, "Operation: %s, Value: %d", op, val);
    let fmt = b"Operation: %s, Value: %d\0";
    libc::snprintf(
        str_ptr,
        64,
        fmt.as_ptr() as *const c_char,
        op,
        val,
    );
    str_ptr
}

/// Equivalent to: int check_permissions(int perms, int required)
#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    ((perms & required) == required) as c_int
}

/// Equivalent to: int safe_add(int a, int b, int perms)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        let msg = b"Insufficient permissions for addition\n\0";
        libc::printf(msg.as_ptr() as *const c_char);
        return 0;
    }
    a.wrapping_add(b)
}

/// Equivalent to: int multiply_with_log(int a, int b, char** log_msg)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_log(
    a: c_int,
    b: c_int,
    log_msg: *mut *mut c_char,
) -> c_int {
    let op = b"multiply\0";
    *log_msg = create_result_string(op.as_ptr() as *const c_char, a.wrapping_mul(b));
    if (*log_msg).is_null() {
        return 0;
    }
    a.wrapping_mul(b)
}

/// Equivalent to: int copy_and_sum(int* src, int count)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        let msg = b"Source pointer is NULL\n\0";
        libc::printf(msg.as_ptr() as *const c_char);
        return -1;
    }

    // int* dest = (int*)malloc(count * sizeof(int));
    // Note: matches C behavior — count may be negative, which is undefined in C.
    let total_bytes = (count as usize).wrapping_mul(std::mem::size_of::<c_int>());
    let dest = libc::malloc(total_bytes) as *mut c_int;
    if dest.is_null() {
        let msg = b"Memory allocation failed\n\0";
        libc::printf(msg.as_ptr() as *const c_char);
        return -1;
    }

    libc::memcpy(
        dest as *mut libc::c_void,
        src as *const libc::c_void,
        total_bytes,
    );

    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(*dest.offset(i as isize));
        i += 1;
    }

    libc::free(dest as *mut libc::c_void);
    sum
}

/// Equivalent to: int compare_operations(const char* op1, const char* op2)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(
    op1: *const c_char,
    op2: *const c_char,
) -> c_int {
    if op1.is_null() || op2.is_null() {
        let msg = b"One or both operation strings are NULL\n\0";
        libc::printf(msg.as_ptr() as *const c_char);
        return -1;
    }
    libc::strcmp(op1, op2)
}

/// Equivalent to: int complexmode(int mode, int value1, int value2, int value3)
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

    let permissions: c_int = 0o644;

    let res_tracker = libc::malloc(std::mem::size_of::<Result>()) as *mut Result;
    if res_tracker.is_null() {
        let msg = b"Failed to allocate result tracker\n\0";
        libc::printf(msg.as_ptr() as *const c_char);
        return -1;
    }

    (*res_tracker).value = 0;
    (*res_tracker).permissions = permissions;
    // strcpy(res_tracker->operation, "none");
    let none = b"none\0";
    libc::strcpy(
        (*res_tracker).operation.as_mut_ptr(),
        none.as_ptr() as *const c_char,
    );

    match mode {
        1 => {
            let s = b"addition\0";
            libc::strcpy(
                (*res_tracker).operation.as_mut_ptr(),
                s.as_ptr() as *const c_char,
            );
            result = safe_add(value1, value2, permissions);
            (*res_tracker).value = result;

            let m1 = b"Mode 1: Addition\n\0";
            libc::printf(m1.as_ptr() as *const c_char);
            let m2 = b"Result: %d\n\0";
            libc::printf(m2.as_ptr() as *const c_char, result);
        }
        2 => {
            let s = b"multiplication\0";
            libc::strcpy(
                (*res_tracker).operation.as_mut_ptr(),
                s.as_ptr() as *const c_char,
            );
            result = multiply_with_log(value1, value2, &mut log_message);
            (*res_tracker).value = result;

            let empty = b"\0";
            if log_message.is_null()
                || libc::strcmp(log_message, empty.as_ptr() as *const c_char) == 0
            {
                let m = b"Log message creation failed\n\0";
                libc::printf(m.as_ptr() as *const c_char);
            } else {
                let m = b"Mode 2: %s\n\0";
                libc::printf(m.as_ptr() as *const c_char, log_message);
                libc::free(log_message as *mut libc::c_void);
            }
        }
        3 => {
            let s = b"array_sum\0";
            libc::strcpy(
                (*res_tracker).operation.as_mut_ptr(),
                s.as_ptr() as *const c_char,
            );
            let mut values: [c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(values.as_mut_ptr(), 3);
            (*res_tracker).value = result;

            let m1 = b"Mode 3: Array Sum\n\0";
            libc::printf(m1.as_ptr() as *const c_char);
            let m2 = b"Result: %d\n\0";
            libc::printf(m2.as_ptr() as *const c_char, result);
        }
        4 => {
            let s = b"complex\0";
            libc::strcpy(
                (*res_tracker).operation.as_mut_ptr(),
                s.as_ptr() as *const c_char,
            );

            if check_permissions(permissions, 0o100) != 0 {
                result = (value1.wrapping_mul(value2)).wrapping_add(value3);
            } else {
                result = value1.wrapping_add(value2).wrapping_add(value3);
            }

            (*res_tracker).value = result;
            let m1 = b"Mode 4: Complex Calculation\n\0";
            libc::printf(m1.as_ptr() as *const c_char);
            let m2 = b"Result: %d\n\0";
            libc::printf(m2.as_ptr() as *const c_char, result);
        }
        _ => {
            let m = b"Invalid mode\n\0";
            libc::printf(m.as_ptr() as *const c_char);
            result = -1;
        }
    }

    let none = b"none\0";
    if libc::strcmp(
        (*res_tracker).operation.as_ptr(),
        none.as_ptr() as *const c_char,
    ) != 0
    {
        let m = b"Operation performed: %s\n\0";
        libc::printf(
            m.as_ptr() as *const c_char,
            (*res_tracker).operation.as_ptr(),
        );
    }

    libc::free(res_tracker as *mut libc::c_void);

    result
}
