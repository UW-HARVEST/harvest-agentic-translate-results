// Translation of c_src/src/lib.c — must produce byte-identical output for the
// same inputs.

use std::ffi::c_char;
use std::ffi::c_int;

// We use libc functions directly to ensure that any output written to stdout
// matches the C version byte-for-byte (same buffering, same printf
// implementation).
extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn malloc(size: usize) -> *mut c_char;
    fn free(ptr: *mut c_char);
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers: [c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut c_char = std::ptr::null_mut();
    let mut result: c_int = 0;

    // The original C code defines two identical local string literals and
    // strncmp's them. The branch is never taken, but we preserve the call
    // exactly to keep behavior (and any side effects) byte-identical.
    let expected_str = b"VALID\0".as_ptr() as *const c_char;
    let input_str = b"VALID\0".as_ptr() as *const c_char;
    let validation_failed = unsafe {
        strncmp(input_str, expected_str, strlen(expected_str)) != 0
    };

    'cleanup_block: {
        if validation_failed {
            unsafe {
                printf(b"Input string validation failed.\n\0".as_ptr() as *const c_char);
            }
            break 'cleanup_block;
        }

        for i in 0..4 {
            // Replicate the exact C switch fallthrough semantics:
            //   case 10: result += 10;          // falls through
            //   case 20: result += 20; break;
            //   case 30: result += 30;          // falls through
            //   case 40: result += 40; break;
            //   default: result += numbers[i]; break;
            match numbers[i as usize] {
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
                other => {
                    result += other;
                }
            }
        }

        dynamic_str = unsafe { malloc(50 * std::mem::size_of::<c_char>()) };
        if dynamic_str.is_null() {
            unsafe {
                printf(b"Memory allocation failed.\n\0".as_ptr() as *const c_char);
            }
            break 'cleanup_block;
        }

        // The C code uses TO_STRING(numbers), which stringifies the token
        // `numbers` to the literal "numbers".
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

    // cleanup label
    unsafe { cleanup_resources(dynamic_str) };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_result(label: *const c_char, result: c_int) {
    unsafe {
        printf(b"%s: %d\n\0".as_ptr() as *const c_char, label, result);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe { free(dynamic_str) };
        // The original C code reassigns the local parameter to NULL, which
        // has no observable effect outside the function. We mirror that
        // (no-op) behavior.
    }
}
