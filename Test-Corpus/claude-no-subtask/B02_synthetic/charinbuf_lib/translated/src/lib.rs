// Rust port of c_src/src/lib.c
// Produces byte-identical output to the original C library.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uint;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicI32, Ordering};

const UINT16_MAX: c_uint = 65535;

static COUNTER: AtomicI32 = AtomicI32::new(0);

type OperationFunc = fn(c_int) -> c_int;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
}

fn increment_counter(value: c_int) -> c_int {
    let new_val = COUNTER.load(Ordering::SeqCst).wrapping_add(value);
    COUNTER.store(new_val, Ordering::SeqCst);
    new_val
}

fn decrement_counter(value: c_int) -> c_int {
    let new_val = COUNTER.load(Ordering::SeqCst).wrapping_sub(value);
    COUNTER.store(new_val, Ordering::SeqCst);
    new_val
}

fn multiply_counter(value: c_int) -> c_int {
    let new_val = COUNTER.load(Ordering::SeqCst).wrapping_mul(value);
    COUNTER.store(new_val, Ordering::SeqCst);
    new_val
}

fn reset_counter(value: c_int) -> c_int {
    COUNTER.store(value, Ordering::SeqCst);
    value
}

fn is_string_empty(s: *const c_char) -> c_int {
    if s.is_null() {
        return 1;
    }
    unsafe {
        if *s != 0 {
            return 0;
        }
    }
    1
}

fn find_char_in_buffer(buffer: *const c_char, size: usize, target: c_char) -> *mut c_char {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { memchr(buffer as *const c_void, target as c_int, size) as *mut c_char }
}

fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let len = strlen(initial);
        let buffer = malloc(len + 1) as *mut c_char;
        if !buffer.is_null() {
            strcpy(buffer, initial);
        }
        buffer
    }
}

fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if (value as i64) > (UINT16_MAX as i64) {
        return 0;
    }
    1
}

fn apply_operation(op: Option<OperationFunc>, value: c_int) -> c_int {
    match op {
        None => -1,
        Some(f) => f(value),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result: c_int = 0;
    let buffer: *mut c_char;
    let found_pos: *mut c_char;
    let test_string = b"\0".as_ptr() as *const c_char;
    let non_empty_string = b"Hello, World!\0".as_ptr() as *const c_char;

    let mut current_op: Option<OperationFunc>;

    COUNTER.store(0, Ordering::SeqCst);

    unsafe {
        match mode {
            0 => {
                printf(b"Mode 0: UINT16_MAX validation\n\0".as_ptr() as *const c_char);
                printf(
                    b"Checking if value %d is within uint16_t range...\n\0".as_ptr()
                        as *const c_char,
                    value,
                );

                if validate_uint16_range(value) != 0 {
                    printf(
                        b"Value %d is valid (0 <= value <= %u)\n\0".as_ptr() as *const c_char,
                        value,
                        UINT16_MAX,
                    );
                    result = value;
                } else {
                    printf(
                        b"Value %d is out of range for uint16_t\n\0".as_ptr() as *const c_char,
                        value,
                    );
                    result = -1;
                }

                printf(
                    b"UINT16_MAX constant value: %u\n\0".as_ptr() as *const c_char,
                    UINT16_MAX,
                );
            }
            1 => {
                printf(
                    b"Mode 1: String empty check by dereference\n\0".as_ptr() as *const c_char,
                );

                if is_string_empty(test_string) != 0 {
                    printf(
                        b"Test string is empty (checked with *string)\n\0".as_ptr()
                            as *const c_char,
                    );
                    result = 0;
                } else {
                    printf(b"Test string is not empty\n\0".as_ptr() as *const c_char);
                    result = 1;
                }

                if is_string_empty(non_empty_string) != 0 {
                    printf(b"Non-empty string check failed!\n\0".as_ptr() as *const c_char);
                } else {
                    printf(
                        b"Non-empty string correctly identified\n\0".as_ptr() as *const c_char,
                    );
                    result += 10;
                }
            }
            2 => {
                printf(
                    b"Mode 2: Dynamic memory allocation and free\n\0".as_ptr() as *const c_char,
                );

                buffer = create_buffer(b"Testing malloc and free\0".as_ptr() as *const c_char);

                if !buffer.is_null() {
                    printf(
                        b"Buffer allocated: '%s'\n\0".as_ptr() as *const c_char,
                        buffer,
                    );
                    let buf_len = strlen(buffer);
                    printf(
                        b"Buffer length: %zu\n\0".as_ptr() as *const c_char,
                        buf_len,
                    );
                    result = buf_len as c_int;

                    free(buffer as *mut c_void);
                    printf(b"Buffer freed successfully\n\0".as_ptr() as *const c_char);
                    let _ = buffer; // emulate buffer = NULL;
                } else {
                    printf(b"Failed to allocate buffer\n\0".as_ptr() as *const c_char);
                    result = -1;
                }
            }
            3 => {
                printf(
                    b"Mode 3: Function pointers with static counter\n\0".as_ptr() as *const c_char,
                );

                current_op = Some(reset_counter as OperationFunc);
                result = apply_operation(current_op, value);
                printf(
                    b"Counter reset to: %d\n\0".as_ptr() as *const c_char,
                    result,
                );

                current_op = Some(increment_counter as OperationFunc);
                result = apply_operation(current_op, opt1);
                printf(
                    b"Counter after increment by %d: %d\n\0".as_ptr() as *const c_char,
                    opt1,
                    result,
                );

                current_op = Some(multiply_counter as OperationFunc);
                result = apply_operation(current_op, opt2);
                printf(
                    b"Counter after multiply by %d: %d\n\0".as_ptr() as *const c_char,
                    opt2,
                    result,
                );

                current_op = Some(decrement_counter as OperationFunc);
                result = apply_operation(current_op, 5);
                printf(
                    b"Counter after decrement by 5: %d\n\0".as_ptr() as *const c_char,
                    result,
                );

                printf(
                    b"Final static counter value: %d\n\0".as_ptr() as *const c_char,
                    COUNTER.load(Ordering::SeqCst),
                );
                let _ = current_op;
            }
            4 => {
                printf(
                    b"Mode 4: Using memchr to find character\n\0".as_ptr() as *const c_char,
                );

                buffer = create_buffer(
                    b"Search for character X in this buffer\0".as_ptr() as *const c_char,
                );

                if !buffer.is_null() {
                    let buf_size = strlen(buffer);
                    let search_char: c_char = b'X' as c_char;

                    printf(
                        b"Searching for '%c' in: '%s'\n\0".as_ptr() as *const c_char,
                        search_char as c_int,
                        buffer,
                    );
                    found_pos = find_char_in_buffer(buffer, buf_size, search_char);

                    if !found_pos.is_null() {
                        result = (found_pos as isize - buffer as isize) as c_int;
                        printf(
                            b"Found '%c' at position: %d\n\0".as_ptr() as *const c_char,
                            search_char as c_int,
                            result,
                        );
                    } else {
                        printf(
                            b"Character '%c' not found\n\0".as_ptr() as *const c_char,
                            search_char as c_int,
                        );
                        result = -1;
                    }

                    free(buffer as *mut c_void);
                    let _ = buffer;
                }
            }
            _ => {
                printf(
                    b"Invalid mode: %d\n\0".as_ptr() as *const c_char,
                    mode,
                );
                result = -1;
            }
        }
    }

    result
}
