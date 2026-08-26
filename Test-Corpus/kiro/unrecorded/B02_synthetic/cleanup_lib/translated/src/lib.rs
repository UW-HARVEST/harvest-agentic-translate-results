use std::ffi::{c_char, c_int};
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers = [a, b, c, d];
    let mut result: c_int = 0;

    let expected_str = b"VALID\0";
    let input_str = b"VALID\0";
    if unsafe { libc::strncmp(input_str.as_ptr() as *const c_char, expected_str.as_ptr() as *const c_char, 5) } != 0 {
        unsafe { libc::printf(b"Input string validation failed.\n\0".as_ptr() as *const c_char) };
        cleanup_resources(ptr::null_mut());
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

    let dynamic_str = unsafe { libc::malloc(50) as *mut c_char };
    if dynamic_str.is_null() {
        unsafe { libc::printf(b"Memory allocation failed.\n\0".as_ptr() as *const c_char) };
        cleanup_resources(ptr::null_mut());
        return result;
    }

    unsafe {
        libc::snprintf(dynamic_str, 50, b"Processed numbers: %s\0".as_ptr() as *const c_char, b"numbers\0".as_ptr() as *const c_char);
        libc::printf(b"%s\n\0".as_ptr() as *const c_char, dynamic_str);
    }

    cleanup_resources(dynamic_str);
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn print_result(label: *const c_char, result: c_int) {
    unsafe { libc::printf(b"%s: %d\n\0".as_ptr() as *const c_char, label, result) };
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe { libc::free(dynamic_str as *mut libc::c_void) };
    }
}
