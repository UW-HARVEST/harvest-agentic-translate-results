// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn malloc(size: usize) -> *mut core::ffi::c_void;
    fn free(ptr: *mut core::ffi::c_void);
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe {
            free(dynamic_str as *mut core::ffi::c_void);
        }
        // Note: Setting the local pointer to NULL has no effect on the caller
        // (matches C behavior where the parameter is passed by value).
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers: [c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut c_char = core::ptr::null_mut();
    let mut result: c_int = 0;

    // String validation block — uses C strncmp/strlen for byte-identical behavior.
    let expected_str = b"VALID\0".as_ptr() as *const c_char;
    let input_str = b"VALID\0".as_ptr() as *const c_char;

    let validation_failed = unsafe {
        strncmp(input_str, expected_str, strlen(expected_str)) != 0
    };

    'cleanup: {
        if validation_failed {
            unsafe {
                printf(b"Input string validation failed.\n\0".as_ptr() as *const c_char);
            }
            break 'cleanup;
        }

        for i in 0..4 {
            // Replicate C switch fallthrough semantics exactly.
            match numbers[i] {
                10 => {
                    result += 10;
                    // fallthrough to 20
                    result += 20;
                    // break
                }
                20 => {
                    result += 20;
                }
                30 => {
                    result += 30;
                    // fallthrough to 40
                    result += 40;
                    // break
                }
                40 => {
                    result += 40;
                }
                _ => {
                    result += numbers[i];
                }
            }
        }

        dynamic_str = unsafe { malloc(50 * core::mem::size_of::<c_char>()) as *mut c_char };
        if dynamic_str.is_null() {
            unsafe {
                printf(b"Memory allocation failed.\n\0".as_ptr() as *const c_char);
            }
            break 'cleanup;
        }

        // TO_STRING(numbers) expands to the literal C string "numbers"
        unsafe {
            snprintf(
                dynamic_str,
                50,
                b"Processed numbers: %s\0".as_ptr() as *const c_char,
                b"numbers\0".as_ptr() as *const c_char,
            );
            printf(b"%s\n\0".as_ptr() as *const c_char, dynamic_str);
        }
    }

    cleanup_resources(dynamic_str);
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn print_result(label: *const c_char, result: c_int) {
    unsafe {
        printf(b"%s: %d\n\0".as_ptr() as *const c_char, label, result);
    }
}
