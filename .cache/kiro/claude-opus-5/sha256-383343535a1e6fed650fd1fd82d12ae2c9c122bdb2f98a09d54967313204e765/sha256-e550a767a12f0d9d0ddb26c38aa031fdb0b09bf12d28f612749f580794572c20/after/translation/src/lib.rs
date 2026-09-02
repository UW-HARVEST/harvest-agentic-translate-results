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
use std::ptr;

// The C translation unit uses the platform C library for all I/O and memory
// management. We bind to the very same functions so that stdio buffering,
// formatting and heap ownership are byte-for-byte identical to the original,
// and so that a pointer handed to `cleanup_resources` may legitimately come
// from (or go to) C `malloc`/`free`.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

/// ```c
/// int cleanup(int a, int b, int c, int d);
/// ```
///
/// Faithful translation, including:
///   * the deliberate `switch` fall-through from `case 10` into `case 20`
///     and from `case 30` into `case 40`;
///   * `TO_STRING(numbers)`, which the C preprocessor stringizes to the
///     literal text `numbers` (it is *not* the array contents);
///   * the `goto cleanup` control flow, modelled with a labelled block.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers: [c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut c_char = ptr::null_mut();
    let mut result: c_int = 0;

    let expected_str: *const c_char = c"VALID".as_ptr();
    let input_str: *const c_char = c"VALID".as_ptr();

    // `'cleanup: { ... break 'cleanup; }` reproduces `goto cleanup;`.
    'cleanup: {
        if unsafe { strncmp(input_str, expected_str, strlen(expected_str)) } != 0 {
            unsafe { printf(c"Input string validation failed.\n".as_ptr()) };
            break 'cleanup;
        }

        for i in 0..4usize {
            match numbers[i] {
                10 => {
                    // case 10: falls through into case 20
                    result = result.wrapping_add(10);
                    result = result.wrapping_add(20);
                    // break
                }
                20 => {
                    result = result.wrapping_add(20);
                    // break
                }
                30 => {
                    // case 30: falls through into case 40
                    result = result.wrapping_add(30);
                    result = result.wrapping_add(40);
                    // break
                }
                40 => {
                    result = result.wrapping_add(40);
                    // break
                }
                _ => {
                    result = result.wrapping_add(numbers[i]);
                    // break
                }
            }
        }

        dynamic_str = unsafe { malloc(50 * core::mem::size_of::<c_char>()) } as *mut c_char;
        if dynamic_str.is_null() {
            unsafe { printf(c"Memory allocation failed.\n".as_ptr()) };
            break 'cleanup;
        }

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

/// ```c
/// void print_result(const char *label, int result);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_result(label: *const c_char, result: c_int) {
    unsafe { printf(c"%s: %d\n".as_ptr(), label, result) };
}

/// ```c
/// void cleanup_resources(char *dynamic_str);
/// ```
///
/// The trailing `dynamic_str = NULL;` in the C source only clears the local
/// parameter copy, so it has no observable effect and is intentionally left
/// unreproduced (a comment marks where it was).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe { free(dynamic_str as *mut c_void) };
        // dynamic_str = NULL;  /* dead store on the local parameter */
    }
}
