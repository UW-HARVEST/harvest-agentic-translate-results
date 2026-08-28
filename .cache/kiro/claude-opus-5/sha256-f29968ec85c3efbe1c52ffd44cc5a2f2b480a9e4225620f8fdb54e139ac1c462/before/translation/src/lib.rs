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

use std::ffi::{c_char, c_int, c_ulong, c_void};

// libc functions used so that stdout buffering, allocation and output bytes
// match the original C translation unit exactly.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> c_ulong;
    fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
}

/// `int cleanup(int a, int b, int c, int d);`
#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers: [c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut c_char = std::ptr::null_mut();
    let mut result: c_int = 0;

    // const char *expected_str = "VALID";
    // const char *input_str = "VALID";
    let expected_str = c"VALID".as_ptr();
    let input_str = c"VALID".as_ptr();

    // The C code compares two identical literals, so this branch is never
    // taken; it is reproduced faithfully nonetheless.
    let validation_failed =
        unsafe { strncmp(input_str, expected_str, strlen(expected_str) as usize) } != 0;

    if validation_failed {
        unsafe {
            printf(c"Input string validation failed.\n".as_ptr());
        }
        // goto cleanup;
    } else {
        for i in 0..4usize {
            // Reproduces the C switch, including the intentional
            // fall-through from `case 10` into `case 20` and from
            // `case 30` into `case 40`.
            match numbers[i] {
                10 => {
                    result = result.wrapping_add(10);
                    // fall through
                    result = result.wrapping_add(20);
                }
                20 => {
                    result = result.wrapping_add(20);
                }
                30 => {
                    result = result.wrapping_add(30);
                    // fall through
                    result = result.wrapping_add(40);
                }
                40 => {
                    result = result.wrapping_add(40);
                }
                n => {
                    result = result.wrapping_add(n);
                }
            }
        }

        dynamic_str = unsafe { malloc(50 * std::mem::size_of::<c_char>()) } as *mut c_char;
        if dynamic_str.is_null() {
            unsafe {
                printf(c"Memory allocation failed.\n".as_ptr());
            }
            // goto cleanup;
        } else {
            // TO_STRING(numbers) stringizes the macro argument token, so the
            // formatted text is literally "Processed numbers: numbers".
            unsafe {
                snprintf(
                    dynamic_str,
                    50,
                    c"Processed numbers: %s".as_ptr(),
                    c"numbers".as_ptr(),
                );
                printf(c"%s\n".as_ptr(), dynamic_str);
            }
        }
    }

    // cleanup:
    cleanup_resources(dynamic_str);
    result
}

/// `void print_result(const char *label, int result);`
#[unsafe(no_mangle)]
pub extern "C" fn print_result(label: *const c_char, result: c_int) {
    unsafe {
        printf(c"%s: %d\n".as_ptr(), label, result);
    }
}

/// `void cleanup_resources(char *dynamic_str);`
#[unsafe(no_mangle)]
pub extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe {
            free(dynamic_str as *mut c_void);
        }
        // The C code nulls its local copy of the parameter, which has no
        // observable effect; preserved as a no-op.
    }
}
