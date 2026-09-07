// Rust translation of c_src/src/lib.c (public header: c_src/include/lib.h).
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
//
// Notes on fidelity:
//  * The C translation unit declares no `static` functions, so every function
//    below is part of the shared library's public ABI and is exported with the
//    same linker name (no namespace macros exist in the header).
//  * libc `malloc`/`free`/`memcpy`/`snprintf`/`strcmp`/`printf` are called
//    directly rather than reimplemented, so that heap ownership stays
//    compatible with callers (they may `free()` the returned string), and so
//    that stdout formatting/buffering is byte-for-byte identical to the C
//    library's.
//  * C signed-integer arithmetic is reproduced with wrapping operations so that
//    overflow behaves like the compiled C (two's complement wraparound) instead
//    of panicking.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
}

// #define READ_PERM 0400
const READ_PERM: c_int = 0o400;
// #define WRITE_PERM 0200
const WRITE_PERM: c_int = 0o200;
// #define EXEC_PERM 0100 (declared in the C source; unused there as a macro,
// the literal 0100 is spelled out at its only use site in `complexmode`).
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

// typedef struct { int value; char operation[32]; int permissions; } Result;
#[repr(C)]
struct Result {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

/// Equivalent of `strcpy(dst, src)` for a NUL-terminated byte literal.
///
/// Like C's `strcpy`, this writes only the string bytes plus the terminating
/// NUL and leaves the remainder of the destination buffer untouched.
unsafe fn strcpy_literal(dst: *mut c_char, src: &[u8]) {
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr() as *const c_char, dst, src.len());
        *dst.add(src.len()) = 0;
    }
}

// char* create_result_string(const char* op, int val)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let s = unsafe { malloc(64 * core::mem::size_of::<c_char>()) } as *mut c_char;
    if s.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        snprintf(s, 64, c"Operation: %s, Value: %d".as_ptr(), op, val);
    }
    s
}

// int check_permissions(int perms, int required)
#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    ((perms & required) == required) as c_int
}

// int safe_add(int a, int b, int perms)
#[unsafe(no_mangle)]
pub extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        unsafe {
            printf(c"Insufficient permissions for addition\n".as_ptr());
        }
        return 0;
    }
    a.wrapping_add(b)
}

// int multiply_with_log(int a, int b, char** log_msg)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_log(
    a: c_int,
    b: c_int,
    log_msg: *mut *mut c_char,
) -> c_int {
    // The C code dereferences `log_msg` unconditionally, with no NULL check.
    unsafe {
        *log_msg = create_result_string(c"multiply".as_ptr(), a.wrapping_mul(b));
        if (*log_msg).is_null() {
            return 0;
        }
    }
    a.wrapping_mul(b)
}

// int copy_and_sum(int* src, int count)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        unsafe {
            printf(c"Source pointer is NULL\n".as_ptr());
        }
        return -1;
    }

    // `count * sizeof(int)`: `count` is sign-extended to size_t, then the
    // product wraps modulo 2^64 exactly as it does in C.
    let nbytes = (count as isize as usize).wrapping_mul(core::mem::size_of::<c_int>());

    let dest = unsafe { malloc(nbytes) } as *mut c_int;
    if dest.is_null() {
        unsafe {
            printf(c"Memory allocation failed\n".as_ptr());
        }
        return -1;
    }

    unsafe {
        memcpy(dest as *mut c_void, src as *const c_void, nbytes);
    }

    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(unsafe { *dest.offset(i as isize) });
        i = i.wrapping_add(1);
    }

    unsafe {
        free(dest as *mut c_void);
    }
    sum
}

// int compare_operations(const char* op1, const char* op2)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        unsafe {
            printf(c"One or both operation strings are NULL\n".as_ptr());
        }
        return -1;
    }

    unsafe { strcmp(op1, op2) }
}

// int complexmode(int mode, int value1, int value2, int value3)
//
// `result` is initialized to 0 and then overwritten in every switch arm, just
// as in the C source; the dead initial store is kept for faithfulness.
#[allow(unused_assignments)]
#[unsafe(no_mangle)]
pub extern "C" fn complexmode(
    mode: c_int,
    value1: c_int,
    value2: c_int,
    value3: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let mut log_message: *mut c_char = ptr::null_mut();

    let permissions: c_int = 0o644; // rw-r--r--

    let res_tracker = unsafe { malloc(core::mem::size_of::<Result>()) } as *mut Result;
    if res_tracker.is_null() {
        unsafe {
            printf(c"Failed to allocate result tracker\n".as_ptr());
        }
        return -1;
    }

    let operation = unsafe { (*res_tracker).operation.as_mut_ptr() };

    unsafe {
        (*res_tracker).value = 0;
        (*res_tracker).permissions = permissions;
        strcpy_literal(operation, b"none");
    }

    match mode {
        1 => {
            unsafe { strcpy_literal(operation, b"addition") };
            result = safe_add(value1, value2, permissions);
            unsafe { (*res_tracker).value = result };

            unsafe {
                printf(c"Mode 1: Addition\n".as_ptr());
                printf(c"Result: %d\n".as_ptr(), result);
            }
        }

        2 => {
            unsafe { strcpy_literal(operation, b"multiplication") };
            result = unsafe { multiply_with_log(value1, value2, &mut log_message) };
            unsafe { (*res_tracker).value = result };

            if log_message.is_null() || unsafe { strcmp(log_message, c"".as_ptr()) } == 0 {
                unsafe {
                    printf(c"Log message creation failed\n".as_ptr());
                }
            } else {
                unsafe {
                    printf(c"Mode 2: %s\n".as_ptr(), log_message);
                    free(log_message as *mut c_void);
                }
            }
        }

        3 => {
            unsafe { strcpy_literal(operation, b"array_sum") };
            let mut values: [c_int; 3] = [value1, value2, value3];
            result = unsafe { copy_and_sum(values.as_mut_ptr(), 3) };
            unsafe { (*res_tracker).value = result };

            unsafe {
                printf(c"Mode 3: Array Sum\n".as_ptr());
                printf(c"Result: %d\n".as_ptr(), result);
            }
        }

        4 => {
            unsafe { strcpy_literal(operation, b"complex") };

            if check_permissions(permissions, 0o100) != 0 {
                result = value1.wrapping_mul(value2).wrapping_add(value3);
            } else {
                result = value1.wrapping_add(value2).wrapping_add(value3);
            }

            unsafe { (*res_tracker).value = result };
            unsafe {
                printf(c"Mode 4: Complex Calculation\n".as_ptr());
                printf(c"Result: %d\n".as_ptr(), result);
            }
        }

        _ => {
            unsafe {
                printf(c"Invalid mode\n".as_ptr());
            }
            result = -1;
        }
    }

    if unsafe { strcmp(operation, c"none".as_ptr()) } != 0 {
        unsafe {
            printf(c"Operation performed: %s\n".as_ptr(), operation);
        }
    }

    unsafe {
        free(res_tracker as *mut c_void);
    }

    result
}
