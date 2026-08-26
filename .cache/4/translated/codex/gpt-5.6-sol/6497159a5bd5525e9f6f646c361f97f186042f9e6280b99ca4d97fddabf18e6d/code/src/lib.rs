use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;

const RESULT_FORMAT: &[u8] = b"Operation: %s, Value: %d\0";
const ADD_PERMISSION_ERROR: &[u8] = b"Insufficient permissions for addition\n\0";
const NULL_SOURCE_ERROR: &[u8] = b"Source pointer is NULL\n\0";
const ALLOCATION_ERROR: &[u8] = b"Memory allocation failed\n\0";
const NULL_OPERATION_ERROR: &[u8] = b"One or both operation strings are NULL\n\0";
const TRACKER_ALLOCATION_ERROR: &[u8] = b"Failed to allocate result tracker\n\0";
const MODE_1_MESSAGE: &[u8] = b"Mode 1: Addition\n\0";
const MODE_2_FORMAT: &[u8] = b"Mode 2: %s\n\0";
const MODE_3_MESSAGE: &[u8] = b"Mode 3: Array Sum\n\0";
const MODE_4_MESSAGE: &[u8] = b"Mode 4: Complex Calculation\n\0";
const RESULT_OUTPUT_FORMAT: &[u8] = b"Result: %d\n\0";
const LOG_CREATION_ERROR: &[u8] = b"Log message creation failed\n\0";
const INVALID_MODE_ERROR: &[u8] = b"Invalid mode\n\0";
const OPERATION_OUTPUT_FORMAT: &[u8] = b"Operation performed: %s\n\0";

const NONE: &[u8] = b"none\0";
const ADDITION: &[u8] = b"addition\0";
const MULTIPLICATION: &[u8] = b"multiplication\0";
const ARRAY_SUM: &[u8] = b"array_sum\0";
const COMPLEX: &[u8] = b"complex\0";
const MULTIPLY: &[u8] = b"multiply\0";
const EMPTY: &[u8] = b"\0";

#[repr(C)]
struct Result {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn memcpy(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strcpy(destination: *mut c_char, source: *const c_char) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[inline]
fn chars(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let string = unsafe { malloc(64) }.cast::<c_char>();
    if string.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        snprintf(string, 64, chars(RESULT_FORMAT), op, val);
    }
    string
}

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    c_int::from((perms & required) == required)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        unsafe {
            printf(chars(ADD_PERMISSION_ERROR));
        }
        return 0;
    }

    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_log(a: c_int, b: c_int, log_msg: *mut *mut c_char) -> c_int {
    let product = a.wrapping_mul(b);
    let message = unsafe { create_result_string(chars(MULTIPLY), product) };
    unsafe {
        log_msg.write(message);
    }
    if message.is_null() {
        return 0;
    }

    product
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        unsafe {
            printf(chars(NULL_SOURCE_ERROR));
        }
        return -1;
    }

    let byte_count = (count as usize).wrapping_mul(size_of::<c_int>());
    let destination = unsafe { malloc(byte_count) }.cast::<c_int>();
    if destination.is_null() {
        unsafe {
            printf(chars(ALLOCATION_ERROR));
        }
        return -1;
    }

    unsafe {
        memcpy(
            destination.cast::<c_void>(),
            src.cast::<c_void>(),
            byte_count,
        );
    }

    let mut sum: c_int = 0;
    let mut index: c_int = 0;
    while index < count {
        sum = sum.wrapping_add(unsafe { destination.add(index as usize).read() });
        index += 1;
    }

    unsafe {
        free(destination.cast::<c_void>());
    }
    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        unsafe {
            printf(chars(NULL_OPERATION_ERROR));
        }
        return -1;
    }

    unsafe { strcmp(op1, op2) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn complexmode(
    mode: c_int,
    value1: c_int,
    value2: c_int,
    value3: c_int,
) -> c_int {
    let result: c_int;
    let mut log_message: *mut c_char = ptr::null_mut();
    let permissions: c_int = 0o644;

    let tracker = unsafe { malloc(size_of::<Result>()) }.cast::<Result>();
    if tracker.is_null() {
        unsafe {
            printf(chars(TRACKER_ALLOCATION_ERROR));
        }
        return -1;
    }

    unsafe {
        ptr::addr_of_mut!((*tracker).value).write(0);
        ptr::addr_of_mut!((*tracker).permissions).write(permissions);
        strcpy(
            ptr::addr_of_mut!((*tracker).operation).cast::<c_char>(),
            chars(NONE),
        );
    }

    match mode {
        1 => unsafe {
            strcpy(
                ptr::addr_of_mut!((*tracker).operation).cast::<c_char>(),
                chars(ADDITION),
            );
            result = safe_add(value1, value2, permissions);
            ptr::addr_of_mut!((*tracker).value).write(result);

            printf(chars(MODE_1_MESSAGE));
            printf(chars(RESULT_OUTPUT_FORMAT), result);
        },
        2 => unsafe {
            strcpy(
                ptr::addr_of_mut!((*tracker).operation).cast::<c_char>(),
                chars(MULTIPLICATION),
            );
            result = multiply_with_log(value1, value2, &mut log_message);
            ptr::addr_of_mut!((*tracker).value).write(result);

            if log_message.is_null() || strcmp(log_message, chars(EMPTY)) == 0 {
                printf(chars(LOG_CREATION_ERROR));
            } else {
                printf(chars(MODE_2_FORMAT), log_message);
                free(log_message.cast::<c_void>());
            }
        },
        3 => unsafe {
            strcpy(
                ptr::addr_of_mut!((*tracker).operation).cast::<c_char>(),
                chars(ARRAY_SUM),
            );
            let mut values = [value1, value2, value3];
            result = copy_and_sum(values.as_mut_ptr(), 3);
            ptr::addr_of_mut!((*tracker).value).write(result);

            printf(chars(MODE_3_MESSAGE));
            printf(chars(RESULT_OUTPUT_FORMAT), result);
        },
        4 => unsafe {
            strcpy(
                ptr::addr_of_mut!((*tracker).operation).cast::<c_char>(),
                chars(COMPLEX),
            );

            if check_permissions(permissions, 0o100) != 0 {
                result = value1.wrapping_mul(value2).wrapping_add(value3);
            } else {
                result = value1.wrapping_add(value2).wrapping_add(value3);
            }

            ptr::addr_of_mut!((*tracker).value).write(result);
            printf(chars(MODE_4_MESSAGE));
            printf(chars(RESULT_OUTPUT_FORMAT), result);
        },
        _ => unsafe {
            printf(chars(INVALID_MODE_ERROR));
            result = -1;
        },
    }

    unsafe {
        if strcmp(
            ptr::addr_of!((*tracker).operation).cast::<c_char>(),
            chars(NONE),
        ) != 0
        {
            printf(
                chars(OPERATION_OUTPUT_FORMAT),
                ptr::addr_of!((*tracker).operation).cast::<c_char>(),
            );
        }

        free(tracker.cast::<c_void>());
    }

    result
}
