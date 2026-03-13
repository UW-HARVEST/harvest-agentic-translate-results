use std::ffi::{c_char, c_int};
use std::os::raw::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers = [a, b, c, d];
    let mut dynamic_str: *mut c_char = std::ptr::null_mut();
    let mut result: c_int = 0;

    unsafe {
        let expected_str = b"VALID\0".as_ptr() as *const c_char;
        let input_str = b"VALID\0".as_ptr() as *const c_char;
        if strncmp(input_str, expected_str, strlen(expected_str)) != 0 {
            printf(b"Input string validation failed.\n\0".as_ptr() as *const c_char);
            cleanup_resources(dynamic_str);
            return result;
        }

        for i in 0..4 {
            match numbers[i] {
                10 => {
                    result += 10;
                    // fall-through into case 20
                    result += 20;
                }
                20 => {
                    result += 20;
                }
                30 => {
                    result += 30;
                    // fall-through into case 40
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

        dynamic_str = malloc(50) as *mut c_char;
        if dynamic_str.is_null() {
            printf(b"Memory allocation failed.\n\0".as_ptr() as *const c_char);
            cleanup_resources(dynamic_str);
            return result;
        }

        snprintf(
            dynamic_str,
            50,
            b"Processed numbers: %s\0".as_ptr() as *const c_char,
            b"numbers\0".as_ptr() as *const c_char,
        );
        printf(b"%s\n\0".as_ptr() as *const c_char, dynamic_str);

        cleanup_resources(dynamic_str);
    }
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
    unsafe {
        if !dynamic_str.is_null() {
            free(dynamic_str as *mut c_void);
        }
    }
}
