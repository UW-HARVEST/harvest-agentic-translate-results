// Rust translation of c_src/src/lib.c (MIT Lincoln Laboratory, 2025).
//
// The C library exports exactly three public symbols:
//   cleanup, print_result, cleanup_resources
//
// Behavior is reproduced bit-for-bit, including the switch-statement
// fall-through semantics and the fact that `TO_STRING(numbers)` stringizes
// the *token* `numbers` (so the printed text is literally "numbers").
// libc's stdio/malloc are used directly so that output buffering, ordering
// and formatting are byte-identical to the C build.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
}

/// `int cleanup(int a, int b, int c, int d);`
#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers: [c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut c_char = std::ptr::null_mut();
    let mut result: c_int = 0;

    // const char *expected_str = "VALID";
    // const char *input_str    = "VALID";
    let expected_str: *const c_char = c"VALID".as_ptr();
    let input_str: *const c_char = c"VALID".as_ptr();

    // Single exit path shared by every `goto cleanup;` in the C source.
    'done: {
        if unsafe { strncmp(input_str, expected_str, strlen(expected_str)) } != 0 {
            unsafe { printf(c"Input string validation failed.\n".as_ptr()) };
            break 'done; // goto cleanup;
        }

        for i in 0..4usize {
            match numbers[i] {
                10 => {
                    // case 10: falls through into case 20
                    result = result.wrapping_add(10);
                    result = result.wrapping_add(20);
                }
                20 => {
                    result = result.wrapping_add(20);
                }
                30 => {
                    // case 30: falls through into case 40
                    result = result.wrapping_add(30);
                    result = result.wrapping_add(40);
                }
                40 => {
                    result = result.wrapping_add(40);
                }
                other => {
                    result = result.wrapping_add(other);
                }
            }
        }

        dynamic_str = unsafe { malloc(50 * std::mem::size_of::<c_char>()) } as *mut c_char;
        if dynamic_str.is_null() {
            unsafe { printf(c"Memory allocation failed.\n".as_ptr()) };
            break 'done; // goto cleanup;
        }

        // TO_STRING(numbers) stringizes the macro argument token: "numbers".
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

    // cleanup:
    unsafe { cleanup_resources(dynamic_str) };
    result
}

/// `void print_result(const char *label, int result);`
#[unsafe(no_mangle)]
pub extern "C" fn print_result(label: *const c_char, result: c_int) {
    unsafe { printf(c"%s: %d\n".as_ptr(), label, result) };
}

/// `void cleanup_resources(char *dynamic_str);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe { free(dynamic_str as *mut c_void) };
        // The C code assigns NULL to its local copy of the pointer here,
        // which has no observable effect.
    }
}
