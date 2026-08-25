use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn strncmp(left: *const c_char, right: *const c_char, count: usize) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers = [a, b, c, d];
    let mut result: c_int = 0;

    let expected_str = c"VALID";
    let input_str = c"VALID";
    // SAFETY: Both values are static, null-terminated strings.
    if unsafe {
        strncmp(
            input_str.as_ptr(),
            expected_str.as_ptr(),
            strlen(expected_str.as_ptr()),
        )
    } != 0
    {
        // SAFETY: The argument is a static, null-terminated format string.
        unsafe {
            printf(c"Input string validation failed.\n".as_ptr());
            cleanup_resources(std::ptr::null_mut());
        }
        return result;
    }

    for number in numbers {
        result = result.wrapping_add(match number {
            10 => 30,
            20 => 20,
            30 => 70,
            40 => 40,
            _ => number,
        });
    }

    // SAFETY: The allocation is either freed by cleanup_resources or is null.
    let dynamic_str = unsafe { malloc(50).cast::<c_char>() };
    if dynamic_str.is_null() {
        // SAFETY: The argument is a static, null-terminated format string.
        unsafe {
            printf(c"Memory allocation failed.\n".as_ptr());
        }
    } else {
        // SAFETY: dynamic_str references 50 writable bytes, and all strings are
        // static and null-terminated.
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

    // SAFETY: dynamic_str is null or is the live allocation returned above.
    unsafe {
        cleanup_resources(dynamic_str);
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_result(label: *const c_char, result: c_int) {
    // SAFETY: This deliberately preserves C's contract that label points to a
    // null-terminated string acceptable to printf.
    unsafe {
        printf(c"%s: %d\n".as_ptr(), label, result);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        // SAFETY: This preserves the C API's contract that the pointer came
        // from a compatible allocator and has not already been freed.
        unsafe {
            free(dynamic_str.cast::<c_void>());
        }
    }
}
