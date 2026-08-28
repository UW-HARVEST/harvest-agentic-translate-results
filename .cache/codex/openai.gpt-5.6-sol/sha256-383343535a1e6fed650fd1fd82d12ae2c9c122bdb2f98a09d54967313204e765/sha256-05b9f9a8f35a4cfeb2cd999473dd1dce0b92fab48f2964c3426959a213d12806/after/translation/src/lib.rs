use std::ffi::{c_char, c_int, c_void};
use std::ptr;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn strncmp(left: *const c_char, right: *const c_char, count: usize) -> c_int;
}

const VALID: &[u8] = b"VALID\0";
const VALIDATION_FAILED: &[u8] = b"Input string validation failed.\n\0";
const ALLOCATION_FAILED: &[u8] = b"Memory allocation failed.\n\0";
const PROCESSED_FORMAT: &[u8] = b"Processed numbers: %s\0";
const NUMBERS: &[u8] = b"numbers\0";
const STRING_LINE_FORMAT: &[u8] = b"%s\n\0";
const RESULT_FORMAT: &[u8] = b"%s: %d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers = [a, b, c, d];
    let mut dynamic_str: *mut c_char = ptr::null_mut();
    let mut result: c_int = 0;

    let expected_str = VALID.as_ptr().cast();
    let input_str = VALID.as_ptr().cast();
    // SAFETY: Both pointers refer to static NUL-terminated byte strings.
    if unsafe { strncmp(input_str, expected_str, strlen(expected_str)) } != 0 {
        // SAFETY: The format is a static NUL-terminated string with no conversions.
        unsafe { printf(VALIDATION_FAILED.as_ptr().cast()) };
    } else {
        for number in numbers {
            let increment = match number {
                10 => 30,
                20 => 20,
                30 => 70,
                40 => 40,
                _ => number,
            };
            result = result.wrapping_add(increment);
        }

        // SAFETY: malloc is called with the same fixed size as the C implementation.
        dynamic_str = unsafe { malloc(50) }.cast();
        if dynamic_str.is_null() {
            // SAFETY: The format is a static NUL-terminated string with no conversions.
            unsafe { printf(ALLOCATION_FAILED.as_ptr().cast()) };
        } else {
            // SAFETY: dynamic_str addresses the 50-byte allocation above, and all strings
            // are static and NUL-terminated.
            unsafe {
                snprintf(
                    dynamic_str,
                    50,
                    PROCESSED_FORMAT.as_ptr().cast(),
                    NUMBERS.as_ptr().cast::<c_char>(),
                );
                printf(STRING_LINE_FORMAT.as_ptr().cast(), dynamic_str);
            }
        }
    }

    // SAFETY: dynamic_str is either null or the allocation returned by malloc above.
    unsafe { cleanup_resources(dynamic_str) };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_result(label: *const c_char, result: c_int) {
    // SAFETY: Matching the C ABI, the caller is responsible for supplying a valid C string.
    unsafe { printf(RESULT_FORMAT.as_ptr().cast(), label, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup_resources(mut dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        // SAFETY: Matching the C ABI, a non-null argument must be free-compatible.
        unsafe { free(dynamic_str.cast()) };
        dynamic_str = ptr::null_mut();
        let _ = dynamic_str;
    }
}
