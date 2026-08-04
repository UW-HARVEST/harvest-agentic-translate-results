// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Preserves byte-identical output.

use std::ffi::c_char;
use std::ffi::c_int;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut core::ffi::c_void;
    fn free(ptr: *mut core::ffi::c_void);
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe {
            free(dynamic_str as *mut core::ffi::c_void);
        }
        // The C code reassigns the local pointer to NULL after free(),
        // but that has no observable effect on the caller. We mirror
        // the behavior by doing nothing further here.
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers: [c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut c_char = std::ptr::null_mut();
    let mut result: c_int = 0;

    let expected_str = b"VALID\0".as_ptr() as *const c_char;
    let input_str = b"VALID\0".as_ptr() as *const c_char;

    unsafe {
        if strncmp(input_str, expected_str, strlen(expected_str)) != 0 {
            printf(b"Input string validation failed.\n\0".as_ptr() as *const c_char);
            cleanup_resources(dynamic_str);
            return result;
        }
    }

    // Mirror the C switch-case fall-through semantics exactly.
    for i in 0..4 {
        match numbers[i] {
            10 => {
                result += 10;
                // fall through to case 20
                result += 20;
            }
            20 => {
                result += 20;
            }
            30 => {
                result += 30;
                // fall through to case 40
                result += 40;
            }
            40 => {
                result += 40;
            }
            _ => {
                result += numbers[i];
            }
        }
    }

    unsafe {
        dynamic_str = malloc(50 * std::mem::size_of::<c_char>()) as *mut c_char;
        if dynamic_str.is_null() {
            printf(b"Memory allocation failed.\n\0".as_ptr() as *const c_char);
            cleanup_resources(dynamic_str);
            return result;
        }

        // TO_STRING(numbers) expands to the string literal "numbers".
        snprintf(
            dynamic_str,
            50,
            b"Processed numbers: %s\0".as_ptr() as *const c_char,
            b"numbers\0".as_ptr() as *const c_char,
        );
        printf(b"%s\n\0".as_ptr() as *const c_char, dynamic_str);
    }

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
