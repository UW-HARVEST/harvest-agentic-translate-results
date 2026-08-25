// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT

use std::ffi::{c_char, c_int, c_uint, c_void};

type OperationFunc = Option<unsafe extern "C" fn(c_int) -> c_int>;

static mut COUNTER: c_int = 0;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn memchr(ptr: *const c_void, value: c_int, size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strcpy(destination: *mut c_char, source: *const c_char) -> *mut c_char;
    fn strlen(value: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn increment_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = COUNTER.wrapping_add(value);
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn decrement_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = COUNTER.wrapping_sub(value);
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = COUNTER.wrapping_mul(value);
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn reset_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = value;
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_string_empty(value: *const c_char) -> c_int {
    if value.is_null() {
        return 1;
    }
    if unsafe { *value } != 0 {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_char_in_buffer(
    buffer: *const c_char,
    size: usize,
    target: c_char,
) -> *mut c_char {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { memchr(buffer.cast(), c_int::from(target), size).cast() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return std::ptr::null_mut();
    }

    let length = unsafe { strlen(initial) };
    let buffer = unsafe { malloc(length.wrapping_add(1)) }.cast::<c_char>();
    if !buffer.is_null() {
        unsafe {
            strcpy(buffer, initial);
        }
    }
    buffer
}

#[unsafe(no_mangle)]
pub extern "C" fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > c_int::from(u16::MAX) {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(op: OperationFunc, value: c_int) -> c_int {
    let Some(op) = op else {
        return -1;
    };
    unsafe { op(value) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
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

            let result;
            if validate_uint16_range(value) != 0 {
                unsafe {
                    printf(
                        c"Value %d is valid (0 <= value <= %u)\n".as_ptr(),
                        value,
                        c_uint::from(u16::MAX),
                    );
                }
                result = value;
            } else {
                unsafe {
                    printf(c"Value %d is out of range for uint16_t\n".as_ptr(), value);
                }
                result = -1;
            }

            unsafe {
                printf(
                    c"UINT16_MAX constant value: %u\n".as_ptr(),
                    c_uint::from(u16::MAX),
                );
            }
            result
        }
        1 => {
            unsafe {
                printf(c"Mode 1: String empty check by dereference\n".as_ptr());
            }

            let mut result: c_int;
            if unsafe { is_string_empty(c"".as_ptr()) } != 0 {
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

            if unsafe { is_string_empty(c"Hello, World!".as_ptr()) } != 0 {
                unsafe {
                    printf(c"Non-empty string check failed!\n".as_ptr());
                }
            } else {
                unsafe {
                    printf(c"Non-empty string correctly identified\n".as_ptr());
                }
                result = result.wrapping_add(10);
            }
            result
        }
        2 => {
            unsafe {
                printf(c"Mode 2: Dynamic memory allocation and free\n".as_ptr());
            }

            let buffer = unsafe { create_buffer(c"Testing malloc and free".as_ptr()) };
            if !buffer.is_null() {
                unsafe {
                    printf(c"Buffer allocated: '%s'\n".as_ptr(), buffer);
                }
                let length = unsafe { strlen(buffer) };
                unsafe {
                    printf(c"Buffer length: %zu\n".as_ptr(), length);
                }
                let result = length as c_int;

                unsafe {
                    free(buffer.cast());
                    printf(c"Buffer freed successfully\n".as_ptr());
                }
                result
            } else {
                unsafe {
                    printf(c"Failed to allocate buffer\n".as_ptr());
                }
                -1
            }
        }
        3 => {
            unsafe {
                printf(c"Mode 3: Function pointers with static counter\n".as_ptr());
            }

            let mut result = unsafe { apply_operation(Some(reset_counter), value) };
            unsafe {
                printf(c"Counter reset to: %d\n".as_ptr(), result);
            }

            result = unsafe { apply_operation(Some(increment_counter), opt1) };
            unsafe {
                printf(
                    c"Counter after increment by %d: %d\n".as_ptr(),
                    opt1,
                    result,
                );
            }

            result = unsafe { apply_operation(Some(multiply_counter), opt2) };
            unsafe {
                printf(c"Counter after multiply by %d: %d\n".as_ptr(), opt2, result);
            }

            result = unsafe { apply_operation(Some(decrement_counter), 5) };
            unsafe {
                printf(c"Counter after decrement by 5: %d\n".as_ptr(), result);
                printf(c"Final static counter value: %d\n".as_ptr(), COUNTER);
            }
            result
        }
        4 => {
            unsafe {
                printf(c"Mode 4: Using memchr to find character\n".as_ptr());
            }

            let buffer =
                unsafe { create_buffer(c"Search for character X in this buffer".as_ptr()) };
            if !buffer.is_null() {
                let buffer_size = unsafe { strlen(buffer) };
                let search_char = b'X' as c_char;
                unsafe {
                    printf(
                        c"Searching for '%c' in: '%s'\n".as_ptr(),
                        c_int::from(search_char),
                        buffer,
                    );
                }
                let found = unsafe { find_char_in_buffer(buffer, buffer_size, search_char) };

                let result;
                if !found.is_null() {
                    result = unsafe { found.offset_from(buffer) } as c_int;
                    unsafe {
                        printf(
                            c"Found '%c' at position: %d\n".as_ptr(),
                            c_int::from(search_char),
                            result,
                        );
                    }
                } else {
                    unsafe {
                        printf(
                            c"Character '%c' not found\n".as_ptr(),
                            c_int::from(search_char),
                        );
                    }
                    result = -1;
                }

                unsafe {
                    free(buffer.cast());
                }
                result
            } else {
                0
            }
        }
        _ => {
            unsafe {
                printf(c"Invalid mode: %d\n".as_ptr(), mode);
            }
            -1
        }
    }
}
