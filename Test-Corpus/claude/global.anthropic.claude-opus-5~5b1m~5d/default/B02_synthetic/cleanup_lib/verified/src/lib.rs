// Rust translation of c_src/src/lib.c (public ABI: cleanup, print_result,
// cleanup_resources).
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

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// The C code performs all of its I/O through C stdio (`printf`) and all of its
// heap work through `malloc`/`free`. We bind directly to the same libc entry
// points so that buffering, formatting and allocator behaviour (and therefore
// the emitted bytes) are byte-for-byte identical to the original library.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

// C string literals (NUL terminated) used verbatim by the translated code.
const FMT_S_NL: &[u8] = b"%s\n\0";
const FMT_LABEL_RESULT: &[u8] = b"%s: %d\n\0";
const MSG_INPUT_VALIDATION_FAILED: &[u8] = b"Input string validation failed.\n\0";
const MSG_MALLOC_FAILED: &[u8] = b"Memory allocation failed.\n\0";
const STR_VALID: &[u8] = b"VALID\0";
// snprintf(dynamic_str, 50, "Processed numbers: %s", TO_STRING(numbers))
// TO_STRING(numbers) stringizes the macro argument, yielding the literal
// "numbers" (NOT the contents of the array).
const FMT_PROCESSED: &[u8] = b"Processed numbers: %s\0";
const STRINGIZED_NUMBERS: &[u8] = b"numbers\0";

/// int cleanup(int a, int b, int c, int d);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers: [c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut c_char = ptr::null_mut();
    let mut result: c_int = 0;

    unsafe {
        // `black_box` keeps the two pointers opaque to the optimizer. Without it
        // LLVM proves both operands are the same constant object and folds the
        // `strncmp` call away entirely, which would make this library's
        // observable libc call sequence differ from the C original (gcc emits a
        // real `strlen` + `strncmp` pair here at every optimisation level).
        // The comparison result is unchanged either way.
        let expected_str: *const c_char =
            core::hint::black_box(STR_VALID.as_ptr()) as *const c_char;
        let input_str: *const c_char = core::hint::black_box(STR_VALID.as_ptr()) as *const c_char;
        if strncmp(input_str, expected_str, strlen(expected_str)) != 0 {
            printf(MSG_INPUT_VALIDATION_FAILED.as_ptr() as *const c_char);
            // goto cleanup;
            cleanup_resources(dynamic_str);
            return result;
        }

        for i in 0..4usize {
            // The original switch relies on fall-through:
            //   case 10: result += 10;  /* falls into case 20 */
            //   case 20: result += 20; break;
            //   case 30: result += 30;  /* falls into case 40 */
            //   case 40: result += 40; break;
            //   default: result += numbers[i]; break;
            match numbers[i] {
                10 => {
                    result = result.wrapping_add(10);
                    result = result.wrapping_add(20);
                }
                20 => {
                    result = result.wrapping_add(20);
                }
                30 => {
                    result = result.wrapping_add(30);
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

        dynamic_str = malloc(50 * core::mem::size_of::<c_char>()) as *mut c_char;
        if dynamic_str.is_null() {
            printf(MSG_MALLOC_FAILED.as_ptr() as *const c_char);
            // goto cleanup;
            cleanup_resources(dynamic_str);
            return result;
        }

        snprintf(
            dynamic_str,
            50,
            FMT_PROCESSED.as_ptr() as *const c_char,
            STRINGIZED_NUMBERS.as_ptr() as *const c_char,
        );
        printf(FMT_S_NL.as_ptr() as *const c_char, dynamic_str);

        // cleanup:
        cleanup_resources(dynamic_str);
        result
    }
}

/// void print_result(const char *label, int result);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_result(label: *const c_char, result: c_int) {
    unsafe {
        printf(FMT_LABEL_RESULT.as_ptr() as *const c_char, label, result);
    }
}

/// void cleanup_resources(char *dynamic_str);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe {
            free(dynamic_str as *mut c_void);
        }
        // The original assigns NULL to the local parameter copy, which has no
        // observable effect; reproduced here as a no-op.
    }
}
