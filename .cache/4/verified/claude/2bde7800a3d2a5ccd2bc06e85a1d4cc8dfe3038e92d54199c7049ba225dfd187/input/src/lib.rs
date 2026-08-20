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
// Rust translation of c_src/src/lib.c.
//
// The C library uses the platform C runtime (malloc/free/printf/snprintf/
// strcmp/memcpy).  We bind those same functions directly so that:
//   * heap blocks handed back to callers (create_result_string,
//     multiply_with_log) can be released with the C library's free(),
//   * stdout output is produced through the *same* stdio stream/buffer the C
//     version used, giving byte-identical output and identical interleaving,
//   * strcmp() returns the very same implementation-defined magnitudes.

use core::ffi::{c_char, c_int, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Permission bit macros from lib.c
// ---------------------------------------------------------------------------

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

// ---------------------------------------------------------------------------
// typedef struct { int value; char operation[32]; int permissions; } Result;
// ---------------------------------------------------------------------------

#[repr(C)]
struct Result {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

/// `strcpy(dst, literal)` for the fixed `operation` buffer.  `src` must be a
/// NUL-terminated byte string that fits (as in the C original, where every
/// call site passes a short literal).
unsafe fn strcpy_lit(dst: *mut c_char, src: &[u8]) {
    // `src` includes its terminating NUL byte.
    core::ptr::copy_nonoverlapping(src.as_ptr() as *const c_char, dst, src.len());
}

// ---------------------------------------------------------------------------
// char* create_result_string(const char* op, int val)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let str_ptr = malloc(64 * core::mem::size_of::<c_char>()) as *mut c_char;
    if str_ptr.is_null() {
        return core::ptr::null_mut();
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

// ---------------------------------------------------------------------------
// int check_permissions(int perms, int required)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    ((perms & required) == required) as c_int
}

// ---------------------------------------------------------------------------
// int safe_add(int a, int b, int perms)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// int multiply_with_log(int a, int b, char** log_msg)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_log(
    a: c_int,
    b: c_int,
    log_msg: *mut *mut c_char,
) -> c_int {
    // The C code dereferences `log_msg` unconditionally (no NULL guard); keep
    // that behaviour verbatim.
    *log_msg = create_result_string(
        b"multiply\0".as_ptr() as *const c_char,
        a.wrapping_mul(b),
    );
    if (*log_msg).is_null() {
        return 0;
    }
    a.wrapping_mul(b)
}

// ---------------------------------------------------------------------------
// int copy_and_sum(int* src, int count)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        printf(b"Source pointer is NULL\n\0".as_ptr() as *const c_char);
        return -1;
    }

    // `count * sizeof(int)`: `count` is converted to size_t, so a negative
    // count sign-extends into a huge allocation request (which then fails).
    let nbytes = (count as isize as usize).wrapping_mul(core::mem::size_of::<c_int>());

    let dest = malloc(nbytes) as *mut c_int;
    if dest.is_null() {
        printf(b"Memory allocation failed\n\0".as_ptr() as *const c_char);
        return -1;
    }

    memcpy(dest as *mut c_void, src as *const c_void, nbytes);

    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(*dest.offset(i as isize));
        i += 1;
    }

    free(dest as *mut c_void);
    sum
}

// ---------------------------------------------------------------------------
// int compare_operations(const char* op1, const char* op2)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        printf(b"One or both operation strings are NULL\n\0".as_ptr() as *const c_char);
        return -1;
    }

    strcmp(op1, op2)
}

// ---------------------------------------------------------------------------
// int complexmode(int mode, int value1, int value2, int value3)
// ---------------------------------------------------------------------------

#[allow(unused_assignments)] // mirrors the C `int result = 0;` initialiser
#[unsafe(no_mangle)]
pub unsafe extern "C" fn complexmode(
    mode: c_int,
    value1: c_int,
    value2: c_int,
    value3: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let mut log_message: *mut c_char = core::ptr::null_mut();

    let permissions: c_int = 0o644; // rw-r--r--

    let res_tracker = malloc(core::mem::size_of::<Result>()) as *mut Result;
    if res_tracker.is_null() {
        printf(b"Failed to allocate result tracker\n\0".as_ptr() as *const c_char);
        return -1;
    }

    (*res_tracker).value = 0;
    (*res_tracker).permissions = permissions;
    strcpy_lit((*res_tracker).operation.as_mut_ptr(), b"none\0");

    match mode {
        1 => {
            strcpy_lit((*res_tracker).operation.as_mut_ptr(), b"addition\0");
            result = safe_add(value1, value2, permissions);
            (*res_tracker).value = result;

            printf(b"Mode 1: Addition\n\0".as_ptr() as *const c_char);
            printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
        }

        2 => {
            strcpy_lit((*res_tracker).operation.as_mut_ptr(), b"multiplication\0");
            result = multiply_with_log(value1, value2, &mut log_message);
            (*res_tracker).value = result;

            if log_message.is_null()
                || strcmp(log_message, b"\0".as_ptr() as *const c_char) == 0
            {
                printf(b"Log message creation failed\n\0".as_ptr() as *const c_char);
            } else {
                printf(
                    b"Mode 2: %s\n\0".as_ptr() as *const c_char,
                    log_message as *const c_char,
                );
                free(log_message as *mut c_void);
            }
        }

        3 => {
            strcpy_lit((*res_tracker).operation.as_mut_ptr(), b"array_sum\0");
            let mut values: [c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(values.as_mut_ptr(), 3);
            (*res_tracker).value = result;

            printf(b"Mode 3: Array Sum\n\0".as_ptr() as *const c_char);
            printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
        }

        4 => {
            strcpy_lit((*res_tracker).operation.as_mut_ptr(), b"complex\0");

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
