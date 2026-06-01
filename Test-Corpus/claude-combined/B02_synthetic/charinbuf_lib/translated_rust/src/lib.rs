// Copyright 2025 MIT Lincoln Laboratory
// Translation of c_src/src/lib.c to Rust.
// Uses libc directly to preserve byte-identical stdout (printf) output and
// matching malloc/free/memchr/strlen semantics.

use std::ffi::c_char;
use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

// Static counter shared across counter operations.
// Using AtomicI32 to allow safe interior mutability for FFI access.
// Note: The original C code is not thread-safe; using Relaxed atomic ops
// preserves single-threaded behavior while satisfying Rust's safety rules.
static COUNTER: AtomicI32 = AtomicI32::new(0);

// typedef int (*operation_func)(int);
type OperationFunc = unsafe extern "C" fn(c_int) -> c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn increment_counter(value: c_int) -> c_int {
    let new = COUNTER.load(Ordering::Relaxed).wrapping_add(value);
    COUNTER.store(new, Ordering::Relaxed);
    new
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn decrement_counter(value: c_int) -> c_int {
    let new = COUNTER.load(Ordering::Relaxed).wrapping_sub(value);
    COUNTER.store(new, Ordering::Relaxed);
    new
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_counter(value: c_int) -> c_int {
    let new = COUNTER.load(Ordering::Relaxed).wrapping_mul(value);
    COUNTER.store(new, Ordering::Relaxed);
    new
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn reset_counter(value: c_int) -> c_int {
    COUNTER.store(value, Ordering::Relaxed);
    value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_string_empty(s: *const c_char) -> c_int {
    if s.is_null() {
        return 1;
    }
    if unsafe { *s } != 0 {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_char_in_buffer(
    buffer: *const c_char,
    size: libc::size_t,
    target: c_char,
) -> *mut c_char {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { libc::memchr(buffer as *const libc::c_void, target as c_int, size) as *mut c_char }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return std::ptr::null_mut();
    }

    let len = unsafe { libc::strlen(initial) };
    let buffer = unsafe { libc::malloc(len + 1) } as *mut c_char;

    if !buffer.is_null() {
        unsafe { libc::strcpy(buffer, initial) };
    }

    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > u16::MAX as c_int {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(
    op: Option<OperationFunc>,
    value: c_int,
) -> c_int {
    match op {
        None => -1,
        Some(f) => unsafe { f(value) },
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn charinbuf(
    mode: c_int,
    value: c_int,
    opt1: c_int,
    opt2: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: *mut c_char;
    let found_pos: *mut c_char;
    let test_string = b"\0".as_ptr() as *const c_char;
    let non_empty_string = b"Hello, World!\0".as_ptr() as *const c_char;

    let mut current_op: Option<OperationFunc>;

    COUNTER.store(0, Ordering::Relaxed);

    match mode {
        0 => {
            unsafe {
                libc::printf(b"Mode 0: UINT16_MAX validation\n\0".as_ptr() as *const c_char);
                libc::printf(
                    b"Checking if value %d is within uint16_t range...\n\0".as_ptr()
                        as *const c_char,
                    value,
                );
            }

            if unsafe { validate_uint16_range(value) } != 0 {
                unsafe {
                    libc::printf(
                        b"Value %d is valid (0 <= value <= %u)\n\0".as_ptr() as *const c_char,
                        value,
                        u16::MAX as c_int,
                    );
                }
                result = value;
            } else {
                unsafe {
                    libc::printf(
                        b"Value %d is out of range for uint16_t\n\0".as_ptr() as *const c_char,
                        value,
                    );
                }
                result = -1;
            }

            unsafe {
                libc::printf(
                    b"UINT16_MAX constant value: %u\n\0".as_ptr() as *const c_char,
                    u16::MAX as c_int,
                );
            }
        }
        1 => {
            unsafe {
                libc::printf(
                    b"Mode 1: String empty check by dereference\n\0".as_ptr() as *const c_char,
                );
            }

            if unsafe { is_string_empty(test_string) } != 0 {
                unsafe {
                    libc::printf(
                        b"Test string is empty (checked with *string)\n\0".as_ptr()
                            as *const c_char,
                    );
                }
                result = 0;
            } else {
                unsafe {
                    libc::printf(b"Test string is not empty\n\0".as_ptr() as *const c_char);
                }
                result = 1;
            }

            if unsafe { is_string_empty(non_empty_string) } != 0 {
                unsafe {
                    libc::printf(
                        b"Non-empty string check failed!\n\0".as_ptr() as *const c_char,
                    );
                }
            } else {
                unsafe {
                    libc::printf(
                        b"Non-empty string correctly identified\n\0".as_ptr() as *const c_char,
                    );
                }
                result += 10;
            }
        }
        2 => {
            unsafe {
                libc::printf(
                    b"Mode 2: Dynamic memory allocation and free\n\0".as_ptr() as *const c_char,
                );
            }

            buffer = unsafe {
                create_buffer(b"Testing malloc and free\0".as_ptr() as *const c_char)
            };

            if !buffer.is_null() {
                unsafe {
                    libc::printf(
                        b"Buffer allocated: '%s'\n\0".as_ptr() as *const c_char,
                        buffer,
                    );
                    libc::printf(
                        b"Buffer length: %zu\n\0".as_ptr() as *const c_char,
                        libc::strlen(buffer),
                    );
                    result = libc::strlen(buffer) as c_int;

                    libc::free(buffer as *mut libc::c_void);
                    libc::printf(b"Buffer freed successfully\n\0".as_ptr() as *const c_char);
                }
                buffer = std::ptr::null_mut();
                let _ = buffer;
            } else {
                unsafe {
                    libc::printf(b"Failed to allocate buffer\n\0".as_ptr() as *const c_char);
                }
                result = -1;
            }
        }
        3 => {
            unsafe {
                libc::printf(
                    b"Mode 3: Function pointers with static counter\n\0".as_ptr() as *const c_char,
                );
            }

            current_op = Some(reset_counter);
            result = unsafe { apply_operation(current_op, value) };
            unsafe {
                libc::printf(
                    b"Counter reset to: %d\n\0".as_ptr() as *const c_char,
                    result,
                );
            }

            current_op = Some(increment_counter);
            result = unsafe { apply_operation(current_op, opt1) };
            unsafe {
                libc::printf(
                    b"Counter after increment by %d: %d\n\0".as_ptr() as *const c_char,
                    opt1,
                    result,
                );
            }

            current_op = Some(multiply_counter);
            result = unsafe { apply_operation(current_op, opt2) };
            unsafe {
                libc::printf(
                    b"Counter after multiply by %d: %d\n\0".as_ptr() as *const c_char,
                    opt2,
                    result,
                );
            }

            current_op = Some(decrement_counter);
            result = unsafe { apply_operation(current_op, 5) };
            unsafe {
                libc::printf(
                    b"Counter after decrement by 5: %d\n\0".as_ptr() as *const c_char,
                    result,
                );
            }

            unsafe {
                libc::printf(
                    b"Final static counter value: %d\n\0".as_ptr() as *const c_char,
                    COUNTER.load(Ordering::Relaxed),
                );
            }
        }
        4 => {
            unsafe {
                libc::printf(
                    b"Mode 4: Using memchr to find character\n\0".as_ptr() as *const c_char,
                );
            }

            buffer = unsafe {
                create_buffer(
                    b"Search for character X in this buffer\0".as_ptr() as *const c_char,
                )
            };

            if !buffer.is_null() {
                let buf_size = unsafe { libc::strlen(buffer) };
                let search_char: c_char = b'X' as c_char;

                unsafe {
                    libc::printf(
                        b"Searching for '%c' in: '%s'\n\0".as_ptr() as *const c_char,
                        search_char as c_int,
                        buffer,
                    );
                }
                found_pos = unsafe { find_char_in_buffer(buffer, buf_size, search_char) };

                if !found_pos.is_null() {
                    result = unsafe { found_pos.offset_from(buffer) } as c_int;
                    unsafe {
                        libc::printf(
                            b"Found '%c' at position: %d\n\0".as_ptr() as *const c_char,
                            search_char as c_int,
                            result,
                        );
                    }
                } else {
                    unsafe {
                        libc::printf(
                            b"Character '%c' not found\n\0".as_ptr() as *const c_char,
                            search_char as c_int,
                        );
                    }
                    result = -1;
                }

                unsafe { libc::free(buffer as *mut libc::c_void) };
                buffer = std::ptr::null_mut();
                let _ = buffer;
            }
        }
        _ => {
            unsafe {
                libc::printf(b"Invalid mode: %d\n\0".as_ptr() as *const c_char, mode);
            }
            result = -1;
        }
    }

    result
}
