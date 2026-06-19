use libc::{c_char, c_int, size_t};
use std::ptr;

static mut COUNTER: c_int = 0;

type OperationFunc = unsafe extern "C" fn(c_int) -> c_int;

unsafe extern "C" fn increment_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = COUNTER.wrapping_add(value);
        COUNTER
    }
}

unsafe extern "C" fn decrement_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = COUNTER.wrapping_sub(value);
        COUNTER
    }
}

unsafe extern "C" fn multiply_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = COUNTER.wrapping_mul(value);
        COUNTER
    }
}

unsafe extern "C" fn reset_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = value;
        COUNTER
    }
}

fn is_string_empty(str_ptr: *const c_char) -> c_int {
    if str_ptr.is_null() {
        return 1;
    }

    unsafe {
        if *str_ptr != 0 {
            return 0;
        }
    }

    1
}

fn find_char_in_buffer(buffer: *const c_char, size: size_t, target: c_char) -> *mut c_char {
    if buffer.is_null() {
        return ptr::null_mut();
    }

    unsafe { libc::memchr(buffer.cast(), target as c_int, size).cast() }
}

fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let len = libc::strlen(initial);
        let buffer = libc::malloc(len + 1).cast::<c_char>();

        if !buffer.is_null() {
            libc::strcpy(buffer, initial);
        }

        buffer
    }
}

fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > u16::MAX as c_int {
        return 0;
    }
    1
}

fn apply_operation(op: Option<OperationFunc>, value: c_int) -> c_int {
    match op {
        Some(func) => unsafe { func(value) },
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result: c_int = 0;
    let test_string = c"";
    let non_empty_string = c"Hello, World!";

    unsafe {
        COUNTER = 0;
    }

    match mode {
        0 => unsafe {
            libc::printf(c"Mode 0: UINT16_MAX validation\n".as_ptr());
            libc::printf(
                c"Checking if value %d is within uint16_t range...\n".as_ptr(),
                value,
            );

            if validate_uint16_range(value) != 0 {
                libc::printf(
                    c"Value %d is valid (0 <= value <= %u)\n".as_ptr(),
                    value,
                    u16::MAX as libc::c_uint,
                );
                result = value;
            } else {
                libc::printf(c"Value %d is out of range for uint16_t\n".as_ptr(), value);
                result = -1;
            }

            libc::printf(
                c"UINT16_MAX constant value: %u\n".as_ptr(),
                u16::MAX as libc::c_uint,
            );
        },
        1 => unsafe {
            libc::printf(c"Mode 1: String empty check by dereference\n".as_ptr());

            if is_string_empty(test_string.as_ptr()) != 0 {
                libc::printf(c"Test string is empty (checked with *string)\n".as_ptr());
                result = 0;
            } else {
                libc::printf(c"Test string is not empty\n".as_ptr());
                result = 1;
            }

            if is_string_empty(non_empty_string.as_ptr()) != 0 {
                libc::printf(c"Non-empty string check failed!\n".as_ptr());
            } else {
                libc::printf(c"Non-empty string correctly identified\n".as_ptr());
                result += 10;
            }
        },
        2 => unsafe {
            libc::printf(c"Mode 2: Dynamic memory allocation and free\n".as_ptr());

            let buffer = create_buffer(c"Testing malloc and free".as_ptr());

            if !buffer.is_null() {
                libc::printf(c"Buffer allocated: '%s'\n".as_ptr(), buffer);
                libc::printf(c"Buffer length: %zu\n".as_ptr(), libc::strlen(buffer));
                result = libc::strlen(buffer) as c_int;

                libc::free(buffer.cast());
                libc::printf(c"Buffer freed successfully\n".as_ptr());
            } else {
                libc::printf(c"Failed to allocate buffer\n".as_ptr());
                result = -1;
            }
        },
        3 => unsafe {
            libc::printf(c"Mode 3: Function pointers with static counter\n".as_ptr());

            result = apply_operation(Some(reset_counter), value);
            libc::printf(c"Counter reset to: %d\n".as_ptr(), result);

            result = apply_operation(Some(increment_counter), opt1);
            libc::printf(c"Counter after increment by %d: %d\n".as_ptr(), opt1, result);

            result = apply_operation(Some(multiply_counter), opt2);
            libc::printf(c"Counter after multiply by %d: %d\n".as_ptr(), opt2, result);

            result = apply_operation(Some(decrement_counter), 5);
            libc::printf(c"Counter after decrement by 5: %d\n".as_ptr(), result);

            libc::printf(c"Final static counter value: %d\n".as_ptr(), COUNTER);
        },
        4 => unsafe {
            libc::printf(c"Mode 4: Using memchr to find character\n".as_ptr());

            let buffer = create_buffer(c"Search for character X in this buffer".as_ptr());

            if !buffer.is_null() {
                let buf_size = libc::strlen(buffer);
                let search_char = b'X' as c_char;
                let found_pos;

                libc::printf(
                    c"Searching for '%c' in: '%s'\n".as_ptr(),
                    search_char as c_int,
                    buffer,
                );
                found_pos = find_char_in_buffer(buffer, buf_size, search_char);

                if !found_pos.is_null() {
                    result = found_pos.offset_from(buffer) as c_int;
                    libc::printf(
                        c"Found '%c' at position: %d\n".as_ptr(),
                        search_char as c_int,
                        result,
                    );
                } else {
                    libc::printf(c"Character '%c' not found\n".as_ptr(), search_char as c_int);
                    result = -1;
                }

                libc::free(buffer.cast());
            }
        },
        _ => unsafe {
            libc::printf(c"Invalid mode: %d\n".as_ptr(), mode);
            result = -1;
        },
    }

    result
}
