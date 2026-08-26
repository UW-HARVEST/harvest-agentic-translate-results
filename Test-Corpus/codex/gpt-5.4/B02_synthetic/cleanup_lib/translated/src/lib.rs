use libc::{free, malloc, printf, snprintf, strncmp, strlen};
use std::ffi::{c_char, c_int};
use std::ptr;

const INPUT_VALIDATION_FAILED: &[u8] = b"Input string validation failed.\n\0";
const MEMORY_ALLOCATION_FAILED: &[u8] = b"Memory allocation failed.\n\0";
const PROCESSED_NUMBERS_FMT: &[u8] = b"Processed numbers: %s\0";
const PRINT_STRING_LINE_FMT: &[u8] = b"%s\n\0";
const PRINT_RESULT_FMT: &[u8] = b"%s: %d\n\0";
const VALID: &[u8] = b"VALID\0";
const NUMBERS_STR: &[u8] = b"numbers\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers = [a, b, c, d];
    let mut dynamic_str: *mut c_char = ptr::null_mut();
    let mut result: c_int = 0;

    let expected_str = VALID.as_ptr() as *const c_char;
    let input_str = VALID.as_ptr() as *const c_char;
    if strncmp(input_str, expected_str, strlen(expected_str)) != 0 {
        printf(INPUT_VALIDATION_FAILED.as_ptr() as *const c_char);
        cleanup_resources(dynamic_str);
        return result;
    }

    for number in numbers {
        match number {
            10 => {
                result += 10;
                result += 20;
            }
            20 => {
                result += 20;
            }
            30 => {
                result += 30;
                result += 40;
            }
            40 => {
                result += 40;
            }
            _ => {
                result += number;
            }
        }
    }

    dynamic_str = malloc(50) as *mut c_char;
    if dynamic_str.is_null() {
        printf(MEMORY_ALLOCATION_FAILED.as_ptr() as *const c_char);
        cleanup_resources(dynamic_str);
        return result;
    }

    snprintf(
        dynamic_str,
        50,
        PROCESSED_NUMBERS_FMT.as_ptr() as *const c_char,
        NUMBERS_STR.as_ptr() as *const c_char,
    );
    printf(
        PRINT_STRING_LINE_FMT.as_ptr() as *const c_char,
        dynamic_str as *const c_char,
    );

    cleanup_resources(dynamic_str);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_result(label: *const c_char, result: c_int) {
    printf(PRINT_RESULT_FMT.as_ptr() as *const c_char, label, result);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        free(dynamic_str.cast());
    }
}
