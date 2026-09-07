// Rust translation of c_src/src/lib.c (public API declared in c_src/include/lib.h).
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

#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C code performs all of its output with `printf`, allocates with
// `malloc`/`free` and searches with `memchr`.  We call straight into libc so
// that formatting, stdio buffering (and hence the exact byte stream written to
// stdout) and heap ownership semantics are bit-for-bit identical with the C
// library: buffers returned by `create_buffer` must remain `free()`-able by the
// caller.
// ---------------------------------------------------------------------------
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
}

/// `UINT16_MAX` from `<stdint.h>`.
const UINT16_MAX: c_int = 65535;

/// `static int counter = 0;` — file-scope mutable state shared by the counter
/// operations, exactly as in the C translation unit (no synchronisation, and
/// wrapping arithmetic as produced by the C compiler).
static mut COUNTER: c_int = 0;

/// `typedef int (*operation_func)(int);`
///
/// Modelled as a nullable function pointer so the `if (!op)` check in
/// `apply_operation` can be reproduced faithfully.
pub type OperationFunc = Option<unsafe extern "C" fn(c_int) -> c_int>;

// ---------------------------------------------------------------------------
// Counter operations
// ---------------------------------------------------------------------------

/// `int increment_counter(int value)`
#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = COUNTER.wrapping_add(value);
        COUNTER
    }
}

/// `int decrement_counter(int value)`
#[unsafe(no_mangle)]
pub extern "C" fn decrement_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = COUNTER.wrapping_sub(value);
        COUNTER
    }
}

/// `int multiply_counter(int value)`
#[unsafe(no_mangle)]
pub extern "C" fn multiply_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = COUNTER.wrapping_mul(value);
        COUNTER
    }
}

/// `int reset_counter(int value)`
#[unsafe(no_mangle)]
pub extern "C" fn reset_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = value;
        COUNTER
    }
}

// ---------------------------------------------------------------------------
// String / buffer helpers
// ---------------------------------------------------------------------------

/// `int is_string_empty(const char *str)`
///
/// Returns 1 for a NULL pointer or an empty string, 0 otherwise.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_string_empty(str: *const c_char) -> c_int {
    if str.is_null() {
        return 1;
    }
    if *str != 0 {
        return 0;
    }
    1
}

/// `char *find_char_in_buffer(const char *buffer, size_t size, char target)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_char_in_buffer(
    buffer: *const c_char,
    size: usize,
    target: c_char,
) -> *mut c_char {
    if buffer.is_null() {
        return ptr::null_mut();
    }
    // `target` is promoted to `int` (sign extended, `char` is signed on the
    // reference platform) exactly like the C call.
    memchr(buffer as *const c_void, target as c_int, size) as *mut c_char
}

/// `char *create_buffer(const char *initial)`
///
/// Allocates with `malloc` so the result can be released with `free` by the
/// caller.  A failed allocation is propagated as NULL, and — as in the C — the
/// copy is only performed when the allocation succeeded.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return ptr::null_mut();
    }

    let len = strlen(initial);
    let buffer = malloc(len + 1) as *mut c_char;

    if !buffer.is_null() {
        strcpy(buffer, initial);
    }

    buffer
}

/// `int validate_uint16_range(int value)`
#[unsafe(no_mangle)]
pub extern "C" fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > UINT16_MAX {
        return 0;
    }
    1
}

/// `int apply_operation(operation_func op, int value)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(op: OperationFunc, value: c_int) -> c_int {
    match op {
        None => -1,
        Some(op) => op(value),
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// `int charinbuf(int mode, int value, int opt1, int opt2)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn charinbuf(
    mode: c_int,
    value: c_int,
    opt1: c_int,
    opt2: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: *mut c_char = ptr::null_mut();
    let found_pos: *mut c_char;
    let test_string: *const c_char = c_str(b"\0");
    let non_empty_string: *const c_char = c_str(b"Hello, World!\0");

    let mut current_op: OperationFunc;

    COUNTER = 0;

    match mode {
        0 => {
            printf(c_str(b"Mode 0: UINT16_MAX validation\n\0"));
            printf(
                c_str(b"Checking if value %d is within uint16_t range...\n\0"),
                value,
            );

            if validate_uint16_range(value) != 0 {
                printf(
                    c_str(b"Value %d is valid (0 <= value <= %u)\n\0"),
                    value,
                    UINT16_MAX as c_uint,
                );
                result = value;
            } else {
                printf(c_str(b"Value %d is out of range for uint16_t\n\0"), value);
                result = -1;
            }

            printf(
                c_str(b"UINT16_MAX constant value: %u\n\0"),
                UINT16_MAX as c_uint,
            );
        }

        1 => {
            printf(c_str(b"Mode 1: String empty check by dereference\n\0"));

            if is_string_empty(test_string) != 0 {
                printf(c_str(b"Test string is empty (checked with *string)\n\0"));
                result = 0;
            } else {
                printf(c_str(b"Test string is not empty\n\0"));
                result = 1;
            }

            if is_string_empty(non_empty_string) != 0 {
                printf(c_str(b"Non-empty string check failed!\n\0"));
            } else {
                printf(c_str(b"Non-empty string correctly identified\n\0"));
                result = result.wrapping_add(10);
            }
        }

        2 => {
            printf(c_str(b"Mode 2: Dynamic memory allocation and free\n\0"));

            buffer = create_buffer(c_str(b"Testing malloc and free\0"));

            if !buffer.is_null() {
                printf(c_str(b"Buffer allocated: '%s'\n\0"), buffer);
                printf(c_str(b"Buffer length: %zu\n\0"), strlen(buffer));
                result = strlen(buffer) as c_int;

                free(buffer as *mut c_void);
                printf(c_str(b"Buffer freed successfully\n\0"));
                buffer = ptr::null_mut();
            } else {
                printf(c_str(b"Failed to allocate buffer\n\0"));
                result = -1;
            }
        }

        3 => {
            printf(c_str(b"Mode 3: Function pointers with static counter\n\0"));

            current_op = Some(reset_counter as unsafe extern "C" fn(c_int) -> c_int);
            result = apply_operation(current_op, value);
            printf(c_str(b"Counter reset to: %d\n\0"), result);

            current_op = Some(increment_counter as unsafe extern "C" fn(c_int) -> c_int);
            result = apply_operation(current_op, opt1);
            printf(
                c_str(b"Counter after increment by %d: %d\n\0"),
                opt1,
                result,
            );

            current_op = Some(multiply_counter as unsafe extern "C" fn(c_int) -> c_int);
            result = apply_operation(current_op, opt2);
            printf(
                c_str(b"Counter after multiply by %d: %d\n\0"),
                opt2,
                result,
            );

            current_op = Some(decrement_counter as unsafe extern "C" fn(c_int) -> c_int);
            result = apply_operation(current_op, 5);
            printf(c_str(b"Counter after decrement by 5: %d\n\0"), result);

            printf(c_str(b"Final static counter value: %d\n\0"), COUNTER);
        }

        4 => {
            printf(c_str(b"Mode 4: Using memchr to find character\n\0"));

            buffer = create_buffer(c_str(b"Search for character X in this buffer\0"));

            if !buffer.is_null() {
                let buf_size = strlen(buffer);
                let search_char: c_char = b'X' as c_char;

                printf(
                    c_str(b"Searching for '%c' in: '%s'\n\0"),
                    search_char as c_int,
                    buffer,
                );
                found_pos = find_char_in_buffer(buffer, buf_size, search_char);

                if !found_pos.is_null() {
                    result = found_pos.offset_from(buffer) as c_int;
                    printf(
                        c_str(b"Found '%c' at position: %d\n\0"),
                        search_char as c_int,
                        result,
                    );
                } else {
                    printf(c_str(b"Character '%c' not found\n\0"), search_char as c_int);
                    result = -1;
                }

                free(buffer as *mut c_void);
                buffer = ptr::null_mut();
            }
        }

        _ => {
            printf(c_str(b"Invalid mode: %d\n\0"), mode);
            result = -1;
        }
    }

    // Mirror the C code's unused final assignments so behaviour (and any
    // observable side effects) stay identical.
    let _ = buffer;

    result
}

/// Helper: NUL-terminated byte literal -> `const char *`.
#[inline(always)]
fn c_str<const N: usize>(bytes: &'static [u8; N]) -> *const c_char {
    debug_assert_eq!(bytes[N - 1], 0u8);
    bytes.as_ptr() as *const c_char
}
