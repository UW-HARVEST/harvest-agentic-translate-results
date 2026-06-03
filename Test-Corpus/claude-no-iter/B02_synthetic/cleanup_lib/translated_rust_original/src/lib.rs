// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces the original behavior exactly.

use std::ffi::c_char;
use std::ffi::c_int;

// FFI declarations to libc so output (printf buffering, formatting) is
// byte-identical to the original C program.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_char;
    fn free(ptr: *mut c_char);
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers: [c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut c_char = std::ptr::null_mut();
    let mut result: c_int = 0;

    // Mirror the C code's control flow (which uses `goto cleanup`) by using
    // a labeled block we can break out of, followed by the cleanup section.
    'cleanup: {
        let expected_str = b"VALID\0".as_ptr() as *const c_char;
        let input_str = b"VALID\0".as_ptr() as *const c_char;
        unsafe {
            if strncmp(input_str, expected_str, strlen(expected_str)) != 0 {
                printf(b"Input string validation failed.\n\0".as_ptr() as *const c_char);
                break 'cleanup;
            }
        }

        for i in 0..4 {
            // Faithfully reproduce the C `switch` with fallthroughs.
            match numbers[i] {
                10 => {
                    result += 10;
                    // fallthrough to case 20
                    result += 20;
                    // break;
                }
                20 => {
                    result += 20;
                    // break;
                }
                30 => {
                    result += 30;
                    // fallthrough to case 40
                    result += 40;
                    // break;
                }
                40 => {
                    result += 40;
                    // break;
                }
                _ => {
                    result += numbers[i];
                    // break;
                }
            }
        }

        unsafe {
            dynamic_str = malloc(50 * std::mem::size_of::<c_char>());
            if dynamic_str.is_null() {
                printf(b"Memory allocation failed.\n\0".as_ptr() as *const c_char);
                break 'cleanup;
            }

            // TO_STRING(numbers) in C stringizes the literal token `numbers`,
            // so the format expansion is: "Processed numbers: %s" with arg "numbers".
            snprintf(
                dynamic_str,
                50,
                b"Processed numbers: %s\0".as_ptr() as *const c_char,
                b"numbers\0".as_ptr() as *const c_char,
            );
            printf(b"%s\n\0".as_ptr() as *const c_char, dynamic_str);
        }
    }

    // cleanup:
    cleanup_resources(dynamic_str);
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn print_result(label: *const c_char, result: c_int) {
    unsafe {
        printf(
            b"%s: %d\n\0".as_ptr() as *const c_char,
            label,
            result,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    // Mirrors the C function. The C version assigns NULL to its local
    // parameter copy after freeing, which has no effect on the caller; we
    // simply free the pointer here.
    if !dynamic_str.is_null() {
        unsafe {
            free(dynamic_str);
        }
    }
}
