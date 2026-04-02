use libc::{c_char, c_int, c_void, free, malloc, memchr, printf, size_t, strcpy, strlen};
use std::ptr;

const UINT16_MAX: c_int = 65535;

static mut COUNTER: c_int = 0;

type OperationFunc = Option<unsafe extern "C" fn(c_int) -> c_int>;

unsafe extern "C" fn increment_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER += value;
        COUNTER
    }
}

unsafe extern "C" fn decrement_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER -= value;
        COUNTER
    }
}

unsafe extern "C" fn multiply_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER *= value;
        COUNTER
    }
}

unsafe extern "C" fn reset_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = value;
        COUNTER
    }
}

unsafe fn is_string_empty(str: *const c_char) -> c_int {
    if str.is_null() {
        return 1;
    }
    if *str != 0 {
        return 0;
    }
    1
}

unsafe fn find_char_in_buffer(buffer: *const c_char, size: size_t, target: c_char) -> *mut c_char {
    if buffer.is_null() {
        return ptr::null_mut();
    }
    memchr(buffer as *const c_void, target as c_int, size) as *mut c_char
}

unsafe fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return ptr::null_mut();
    }
    let len = strlen(initial);
    let buffer = malloc(len + 1) as *mut c_char;
    if !buffer.is_null() {
        strcpy(buffer, initial);
    }
    buffer
}

fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > UINT16_MAX {
        return 0;
    }
    1
}

unsafe fn apply_operation(op: OperationFunc, value: c_int) -> c_int {
    match op {
        None => -1,
        Some(f) => f(value),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    unsafe {
        let mut result: c_int = 0;
        let mut buffer: *mut c_char;
        let found_pos: *mut c_char;
        let test_string: *const c_char = b"\0".as_ptr() as *const c_char;
        let non_empty_string: *const c_char = b"Hello, World!\0".as_ptr() as *const c_char;

        let mut current_op: OperationFunc;

        COUNTER = 0;

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
                        UINT16_MAX as libc::c_uint,
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
                    UINT16_MAX as libc::c_uint,
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

                buffer = create_buffer(
                    b"Testing malloc and free\0".as_ptr() as *const c_char,
                );

                if !buffer.is_null() {
                    printf(b"Buffer allocated: '%s'\n\0".as_ptr() as *const c_char, buffer);
                    printf(
                        b"Buffer length: %zu\n\0".as_ptr() as *const c_char,
                        strlen(buffer),
                    );
                    result = strlen(buffer) as c_int;

                    free(buffer as *mut c_void);
                    printf(b"Buffer freed successfully\n\0".as_ptr() as *const c_char);
                    buffer = ptr::null_mut();
                    let _ = buffer;
                } else {
                    printf(b"Failed to allocate buffer\n\0".as_ptr() as *const c_char);
                    result = -1;
                }
            }
            3 => {
                printf(
                    b"Mode 3: Function pointers with static counter\n\0".as_ptr()
                        as *const c_char,
                );

                current_op = Some(reset_counter);
                result = apply_operation(current_op, value);
                printf(b"Counter reset to: %d\n\0".as_ptr() as *const c_char, result);

                current_op = Some(increment_counter);
                result = apply_operation(current_op, opt1);
                printf(
                    b"Counter after increment by %d: %d\n\0".as_ptr() as *const c_char,
                    opt1,
                    result,
                );

                current_op = Some(multiply_counter);
                result = apply_operation(current_op, opt2);
                printf(
                    b"Counter after multiply by %d: %d\n\0".as_ptr() as *const c_char,
                    opt2,
                    result,
                );

                current_op = Some(decrement_counter);
                result = apply_operation(current_op, 5);
                printf(
                    b"Counter after decrement by 5: %d\n\0".as_ptr() as *const c_char,
                    result,
                );

                printf(
                    b"Final static counter value: %d\n\0".as_ptr() as *const c_char,
                    COUNTER,
                );
            }
            4 => {
                printf(
                    b"Mode 4: Using memchr to find character\n\0".as_ptr() as *const c_char,
                );

                buffer = create_buffer(
                    b"Search for character X in this buffer\0".as_ptr() as *const c_char,
                );

                if !buffer.is_null() {
                    let buf_size: size_t = strlen(buffer);
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
                    buffer = ptr::null_mut();
                    let _ = buffer;
                }
            }
            _ => {
                printf(b"Invalid mode: %d\n\0".as_ptr() as *const c_char, mode);
                result = -1;
            }
        }

        result
    }
}
