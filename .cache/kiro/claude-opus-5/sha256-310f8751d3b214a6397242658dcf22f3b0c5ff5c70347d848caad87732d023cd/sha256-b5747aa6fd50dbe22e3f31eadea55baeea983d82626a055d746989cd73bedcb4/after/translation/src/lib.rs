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

// --- C runtime functions -------------------------------------------------
// Used directly so that stdout buffering, `malloc`/`free` ownership across the
// FFI boundary, and `strcmp` return values match the original C library
// byte-for-byte.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
}

// #define READ_PERM 0400
const READ_PERM: c_int = 0o400;
// #define WRITE_PERM 0200
const WRITE_PERM: c_int = 0o200;
// #define EXEC_PERM 0100 (unused in the C source, kept for parity)
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

/// typedef struct { int value; char operation[32]; int permissions; } Result;
#[repr(C)]
struct Result {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

/// Emulates `strcpy(dest, literal)` into a fixed-size C char array.
fn set_operation(dest: &mut [c_char; 32], s: &str) {
    let bytes = s.as_bytes();
    for (slot, b) in dest.iter_mut().zip(bytes.iter()) {
        *slot = *b as c_char;
    }
    dest[bytes.len()] = 0;
}

/// Compares a NUL-terminated C char array against a Rust string, like
/// `strcmp(arr, literal) == 0`.
fn operation_equals(field: &[c_char; 32], s: &str) -> bool {
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if field[i] != *b as c_char {
            return false;
        }
    }
    field[bytes.len()] == 0
}

#[unsafe(no_mangle)]
pub extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let str_ptr = unsafe { malloc(64 * std::mem::size_of::<c_char>()) } as *mut c_char;
    if str_ptr.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        snprintf(
            str_ptr,
            64,
            b"Operation: %s, Value: %d\0".as_ptr() as *const c_char,
            op,
            val,
        );
    }
    str_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    ((perms & required) == required) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        unsafe {
            printf(b"Insufficient permissions for addition\n\0".as_ptr() as *const c_char);
        }
        return 0;
    }
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_log(a: c_int, b: c_int, log_msg: *mut *mut c_char) -> c_int {
    let msg = create_result_string(
        b"multiply\0".as_ptr() as *const c_char,
        a.wrapping_mul(b),
    );
    unsafe {
        *log_msg = msg;
    }
    if msg.is_null() {
        return 0;
    }
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn copy_and_sum(src: *const c_int, count: c_int) -> c_int {
    if src.is_null() {
        unsafe {
            printf(b"Source pointer is NULL\n\0".as_ptr() as *const c_char);
        }
        return -1;
    }

    // `count * sizeof(int)`: count is converted to size_t, so a negative count
    // becomes a huge allocation request (and fails), exactly as in C.
    let bytes = (count as i64 as u64).wrapping_mul(std::mem::size_of::<c_int>() as u64) as usize;

    let dest = unsafe { malloc(bytes) } as *mut c_int;
    if dest.is_null() {
        unsafe {
            printf(b"Memory allocation failed\n\0".as_ptr() as *const c_char);
        }
        return -1;
    }

    unsafe {
        memcpy(dest as *mut c_void, src as *const c_void, bytes);
    }

    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(unsafe { *dest.offset(i as isize) });
        i += 1;
    }

    unsafe {
        free(dest as *mut c_void);
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        unsafe {
            printf(b"One or both operation strings are NULL\n\0".as_ptr() as *const c_char);
        }
        return -1;
    }

    unsafe { strcmp(op1, op2) }
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut log_message: *mut c_char = std::ptr::null_mut();

    let permissions: c_int = 0o644; // rw-r--r--

    let res_tracker = unsafe { malloc(std::mem::size_of::<Result>()) } as *mut Result;
    if res_tracker.is_null() {
        unsafe {
            printf(b"Failed to allocate result tracker\n\0".as_ptr() as *const c_char);
        }
        return -1;
    }
    // Safe view over the freshly allocated tracker.
    let tracker: &mut Result = unsafe {
        std::ptr::write(
            res_tracker,
            Result {
                value: 0,
                operation: [0; 32],
                permissions: 0,
            },
        );
        &mut *res_tracker
    };

    tracker.value = 0;
    tracker.permissions = permissions;
    set_operation(&mut tracker.operation, "none");

    match mode {
        1 => {
            set_operation(&mut tracker.operation, "addition");
            result = safe_add(value1, value2, permissions);
            tracker.value = result;

            unsafe {
                printf(b"Mode 1: Addition\n\0".as_ptr() as *const c_char);
                printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
            }
        }

        2 => {
            set_operation(&mut tracker.operation, "multiplication");
            result = multiply_with_log(value1, value2, &mut log_message);
            tracker.value = result;

            let empty_or_null = log_message.is_null()
                || unsafe { strcmp(log_message, b"\0".as_ptr() as *const c_char) } == 0;
            if empty_or_null {
                unsafe {
                    printf(b"Log message creation failed\n\0".as_ptr() as *const c_char);
                }
            } else {
                unsafe {
                    printf(b"Mode 2: %s\n\0".as_ptr() as *const c_char, log_message);
                    free(log_message as *mut c_void);
                }
            }
        }

        3 => {
            set_operation(&mut tracker.operation, "array_sum");
            let values: [c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(values.as_ptr(), 3);
            tracker.value = result;

            unsafe {
                printf(b"Mode 3: Array Sum\n\0".as_ptr() as *const c_char);
                printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
            }
        }

        4 => {
            set_operation(&mut tracker.operation, "complex");

            if check_permissions(permissions, 0o100) != 0 {
                result = value1.wrapping_mul(value2).wrapping_add(value3);
            } else {
                result = value1.wrapping_add(value2).wrapping_add(value3);
            }

            tracker.value = result;
            unsafe {
                printf(b"Mode 4: Complex Calculation\n\0".as_ptr() as *const c_char);
                printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
            }
        }

        _ => {
            unsafe {
                printf(b"Invalid mode\n\0".as_ptr() as *const c_char);
            }
            result = -1;
        }
    }

    if !operation_equals(&tracker.operation, "none") {
        unsafe {
            printf(
                b"Operation performed: %s\n\0".as_ptr() as *const c_char,
                tracker.operation.as_ptr(),
            );
        }
    }

    unsafe {
        free(res_tracker as *mut c_void);
    }

    result
}
