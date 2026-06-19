use std::ffi::{c_char, c_int, c_void};
use std::ptr;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers = [a, b, c, d];
    let mut dynamic_str: *mut c_char = ptr::null_mut();
    let mut result: c_int = 0;

    let expected_str = c"VALID".as_ptr();
    let input_str = c"VALID".as_ptr();
    if unsafe { strncmp(input_str, expected_str, strlen(expected_str)) } != 0 {
        unsafe {
            printf(c"Input string validation failed.\n".as_ptr());
        }
        unsafe {
            cleanup_resources(dynamic_str);
        }
        return result;
    }

    for number in numbers {
        match number {
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
            _ => {
                result = result.wrapping_add(number);
            }
        }
    }

    dynamic_str = unsafe { malloc(50) as *mut c_char };
    if dynamic_str.is_null() {
        unsafe {
            printf(c"Memory allocation failed.\n".as_ptr());
            cleanup_resources(dynamic_str);
        }
        return result;
    }

    unsafe {
        snprintf(
            dynamic_str,
            50,
            c"Processed numbers: %s".as_ptr(),
            c"numbers".as_ptr(),
        );
        printf(c"%s\n".as_ptr(), dynamic_str);
        cleanup_resources(dynamic_str);
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_result(label: *const c_char, result: c_int) {
    unsafe {
        printf(c"%s: %d\n".as_ptr(), label, result);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe {
            free(dynamic_str.cast::<c_void>());
        }
        let _dynamic_str = ptr::null_mut::<c_char>();
    }
}
