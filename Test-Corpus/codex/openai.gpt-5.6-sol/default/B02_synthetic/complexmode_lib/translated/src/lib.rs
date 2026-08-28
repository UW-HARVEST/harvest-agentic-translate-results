use std::ffi::{c_char, c_int, c_void};
use std::mem;
use std::ptr;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;

#[repr(C)]
struct ResultTracker {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
}

const RESULT_FORMAT: &[u8] = b"Operation: %s, Value: %d\0";
const MULTIPLY: &[u8] = b"multiply\0";
const EMPTY: &[u8] = b"\0";
const NONE: &[u8] = b"none\0";
const ADDITION: &[u8] = b"addition\0";
const MULTIPLICATION: &[u8] = b"multiplication\0";
const ARRAY_SUM: &[u8] = b"array_sum\0";
const COMPLEX: &[u8] = b"complex\0";

const INSUFFICIENT_PERMISSIONS: &[u8] = b"Insufficient permissions for addition\n\0";
const SOURCE_NULL: &[u8] = b"Source pointer is NULL\n\0";
const ALLOCATION_FAILED: &[u8] = b"Memory allocation failed\n\0";
const OPERATIONS_NULL: &[u8] = b"One or both operation strings are NULL\n\0";
const TRACKER_ALLOCATION_FAILED: &[u8] = b"Failed to allocate result tracker\n\0";
const MODE_1: &[u8] = b"Mode 1: Addition\n\0";
const MODE_2: &[u8] = b"Mode 2: %s\n\0";
const MODE_3: &[u8] = b"Mode 3: Array Sum\n\0";
const MODE_4: &[u8] = b"Mode 4: Complex Calculation\n\0";
const RESULT: &[u8] = b"Result: %d\n\0";
const LOG_FAILED: &[u8] = b"Log message creation failed\n\0";
const INVALID_MODE: &[u8] = b"Invalid mode\n\0";
const OPERATION_PERFORMED: &[u8] = b"Operation performed: %s\n\0";

#[inline]
fn c_str(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

unsafe fn copy_operation(dest: *mut c_char, value: &'static [u8]) {
    // The source strings all fit in ResultTracker::operation, as in the C strcpy calls.
    unsafe {
        ptr::copy_nonoverlapping(value.as_ptr().cast::<c_char>(), dest, value.len());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let output = unsafe { malloc(64) }.cast::<c_char>();
    if output.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        snprintf(output, 64, c_str(RESULT_FORMAT), op, val);
    }
    output
}

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    c_int::from((perms & required) == required)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        unsafe {
            printf(c_str(INSUFFICIENT_PERMISSIONS));
        }
        return 0;
    }

    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_log(a: c_int, b: c_int, log_msg: *mut *mut c_char) -> c_int {
    let result = a.wrapping_mul(b);
    unsafe {
        *log_msg = create_result_string(c_str(MULTIPLY), result);
        if (*log_msg).is_null() {
            return 0;
        }
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        unsafe {
            printf(c_str(SOURCE_NULL));
        }
        return -1;
    }

    let byte_count = (count as usize).wrapping_mul(mem::size_of::<c_int>());
    let dest = unsafe { malloc(byte_count) }.cast::<c_int>();
    if dest.is_null() {
        unsafe {
            printf(c_str(ALLOCATION_FAILED));
        }
        return -1;
    }

    unsafe {
        memcpy(dest.cast(), src.cast(), byte_count);
    }

    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        unsafe {
            sum = sum.wrapping_add(*dest.offset(i as isize));
        }
        i += 1;
    }

    unsafe {
        free(dest.cast());
    }
    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        unsafe {
            printf(c_str(OPERATIONS_NULL));
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

    let tracker = unsafe { malloc(mem::size_of::<ResultTracker>()) }.cast::<ResultTracker>();
    if tracker.is_null() {
        unsafe {
            printf(c_str(TRACKER_ALLOCATION_FAILED));
        }
        return -1;
    }

    unsafe {
        (*tracker).value = 0;
        (*tracker).permissions = permissions;
        copy_operation((*tracker).operation.as_mut_ptr(), NONE);
    }

    match mode {
        1 => unsafe {
            copy_operation((*tracker).operation.as_mut_ptr(), ADDITION);
            result = safe_add(value1, value2, permissions);
            (*tracker).value = result;

            printf(c_str(MODE_1));
            printf(c_str(RESULT), result);
        },
        2 => unsafe {
            copy_operation((*tracker).operation.as_mut_ptr(), MULTIPLICATION);
            result = multiply_with_log(value1, value2, &mut log_message);
            (*tracker).value = result;

            if log_message.is_null() || strcmp(log_message, c_str(EMPTY)) == 0 {
                printf(c_str(LOG_FAILED));
            } else {
                printf(c_str(MODE_2), log_message);
                free(log_message.cast());
            }
        },
        3 => unsafe {
            copy_operation((*tracker).operation.as_mut_ptr(), ARRAY_SUM);
            let mut values = [value1, value2, value3];
            result = copy_and_sum(values.as_mut_ptr(), 3);
            (*tracker).value = result;

            printf(c_str(MODE_3));
            printf(c_str(RESULT), result);
        },
        4 => unsafe {
            copy_operation((*tracker).operation.as_mut_ptr(), COMPLEX);

            if check_permissions(permissions, 0o100) != 0 {
                result = value1.wrapping_mul(value2).wrapping_add(value3);
            } else {
                result = value1.wrapping_add(value2).wrapping_add(value3);
            }

            (*tracker).value = result;
            printf(c_str(MODE_4));
            printf(c_str(RESULT), result);
        },
        _ => {
            unsafe {
                printf(c_str(INVALID_MODE));
            }
            result = -1;
        }
    }

    unsafe {
        if strcmp((*tracker).operation.as_ptr(), c_str(NONE)) != 0 {
            printf(c_str(OPERATION_PERFORMED), (*tracker).operation.as_ptr());
        }
        free(tracker.cast());
    }

    result
}
