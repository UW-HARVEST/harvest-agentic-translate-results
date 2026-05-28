use std::ffi::{c_char, c_int, c_void};
use std::ptr;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

static mut COUNTER: c_int = 0;

type OperationFunc = fn(c_int) -> c_int;

fn increment_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER += value;
        COUNTER
    }
}

fn decrement_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER -= value;
        COUNTER
    }
}

fn multiply_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER *= value;
        COUNTER
    }
}

fn reset_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = value;
        COUNTER
    }
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
        return ptr::null_mut();
    }
    unsafe { memchr(buffer as *const c_void, target as c_int, size) as *mut c_char }
}

fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return ptr::null_mut();
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
    if value > 65535 {
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
    let mut buffer: *mut c_char;
    let test_string = c"";
    let non_empty_string = c"Hello, World!";

    unsafe {
        COUNTER = 0;
    }

    match mode {
        0 => {
            unsafe {
                printf(c"Mode 0: UINT16_MAX validation\n".as_ptr());
                printf(
                    c"Checking if value %d is within uint16_t range...\n".as_ptr(),
                    value,
                );
            }
            if validate_uint16_range(value) != 0 {
                unsafe {
                    printf(
                        c"Value %d is valid (0 <= value <= %u)\n".as_ptr(),
                        value,
                        65535u32,
                    );
                }
                result = value;
            } else {
                unsafe {
                    printf(
                        c"Value %d is out of range for uint16_t\n".as_ptr(),
                        value,
                    );
                }
                result = -1;
            }
            unsafe {
                printf(
                    c"UINT16_MAX constant value: %u\n".as_ptr(),
                    65535u32,
                );
            }
        }
        1 => {
            unsafe {
                printf(c"Mode 1: String empty check by dereference\n".as_ptr());
            }
            if is_string_empty(test_string.as_ptr()) != 0 {
                unsafe {
                    printf(c"Test string is empty (checked with *string)\n".as_ptr());
                }
                result = 0;
            } else {
                unsafe {
                    printf(c"Test string is not empty\n".as_ptr());
                }
                result = 1;
            }
            if is_string_empty(non_empty_string.as_ptr()) != 0 {
                unsafe {
                    printf(c"Non-empty string check failed!\n".as_ptr());
                }
            } else {
                unsafe {
                    printf(c"Non-empty string correctly identified\n".as_ptr());
                }
                result += 10;
            }
        }
        2 => {
            unsafe {
                printf(c"Mode 2: Dynamic memory allocation and free\n".as_ptr());
            }
            buffer = create_buffer(c"Testing malloc and free".as_ptr());
            if !buffer.is_null() {
                unsafe {
                    printf(c"Buffer allocated: '%s'\n".as_ptr(), buffer);
                    let len = strlen(buffer);
                    printf(c"Buffer length: %zu\n".as_ptr(), len);
                    result = len as c_int;
                    free(buffer as *mut c_void);
                    printf(c"Buffer freed successfully\n".as_ptr());
                }
                buffer = ptr::null_mut();
                let _ = buffer;
            } else {
                unsafe {
                    printf(c"Failed to allocate buffer\n".as_ptr());
                }
                result = -1;
            }
        }
        3 => {
            unsafe {
                printf(c"Mode 3: Function pointers with static counter\n".as_ptr());
            }

            let mut current_op: Option<OperationFunc> = Some(reset_counter);
            result = apply_operation(current_op, value);
            unsafe {
                printf(c"Counter reset to: %d\n".as_ptr(), result);
            }

            current_op = Some(increment_counter);
            result = apply_operation(current_op, opt1);
            unsafe {
                printf(
                    c"Counter after increment by %d: %d\n".as_ptr(),
                    opt1,
                    result,
                );
            }

            current_op = Some(multiply_counter);
            result = apply_operation(current_op, opt2);
            unsafe {
                printf(
                    c"Counter after multiply by %d: %d\n".as_ptr(),
                    opt2,
                    result,
                );
            }

            current_op = Some(decrement_counter);
            result = apply_operation(current_op, 5);
            unsafe {
                printf(c"Counter after decrement by 5: %d\n".as_ptr(), result);
            }
            let _ = current_op;

            unsafe {
                let final_counter = COUNTER;
                printf(c"Final static counter value: %d\n".as_ptr(), final_counter);
            }
        }
        4 => {
            unsafe {
                printf(c"Mode 4: Using memchr to find character\n".as_ptr());
            }
            buffer = create_buffer(c"Search for character X in this buffer".as_ptr());
            if !buffer.is_null() {
                let buf_size = unsafe { strlen(buffer) };
                let search_char: c_int = b'X' as c_int;
                unsafe {
                    printf(
                        c"Searching for '%c' in: '%s'\n".as_ptr(),
                        search_char,
                        buffer,
                    );
                }
                let found_pos =
                    find_char_in_buffer(buffer, buf_size, search_char as c_char);
                if !found_pos.is_null() {
                    result = (found_pos as isize - buffer as isize) as c_int;
                    unsafe {
                        printf(
                            c"Found '%c' at position: %d\n".as_ptr(),
                            search_char,
                            result,
                        );
                    }
                } else {
                    unsafe {
                        printf(c"Character '%c' not found\n".as_ptr(), search_char);
                    }
                    result = -1;
                }
                unsafe {
                    free(buffer as *mut c_void);
                }
                buffer = ptr::null_mut();
                let _ = buffer;
            }
        }
        _ => {
            unsafe {
                printf(c"Invalid mode: %d\n".as_ptr(), mode);
            }
            result = -1;
        }
    }

    result
}
