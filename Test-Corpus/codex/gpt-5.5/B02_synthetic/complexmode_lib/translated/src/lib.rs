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
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
}

fn cstr(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

unsafe fn set_operation(dest: *mut c_char, src: &'static [u8]) {
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr().cast::<c_char>(), dest, src.len());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let str_ptr = unsafe { malloc(64 * mem::size_of::<c_char>()) }.cast::<c_char>();
    if str_ptr.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        snprintf(
            str_ptr,
            64,
            cstr(b"Operation: %s, Value: %d\0"),
            op,
            val,
        );
    }

    str_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    ((perms & required) == required) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        unsafe {
            printf(cstr(b"Insufficient permissions for addition\n\0"));
        }
        return 0;
    }

    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_log(
    a: c_int,
    b: c_int,
    log_msg: *mut *mut c_char,
) -> c_int {
    let product = a.wrapping_mul(b);
    unsafe {
        *log_msg = create_result_string(cstr(b"multiply\0"), product);
        if (*log_msg).is_null() {
            return 0;
        }
    }

    product
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        unsafe {
            printf(cstr(b"Source pointer is NULL\n\0"));
        }
        return -1;
    }

    let byte_count = (count as usize).wrapping_mul(mem::size_of::<c_int>());
    let dest = unsafe { malloc(byte_count) }.cast::<c_int>();
    if dest.is_null() {
        unsafe {
            printf(cstr(b"Memory allocation failed\n\0"));
        }
        return -1;
    }

    unsafe {
        memcpy(dest.cast::<c_void>(), src.cast::<c_void>(), byte_count);
    }

    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        unsafe {
            sum = sum.wrapping_add(*dest.offset(i as isize));
        }
        i = i.wrapping_add(1);
    }

    unsafe {
        free(dest.cast::<c_void>());
    }
    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        unsafe {
            printf(cstr(b"One or both operation strings are NULL\n\0"));
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

    let res_tracker = unsafe { malloc(mem::size_of::<ResultTracker>()) }.cast::<ResultTracker>();
    if res_tracker.is_null() {
        unsafe {
            printf(cstr(b"Failed to allocate result tracker\n\0"));
        }
        return -1;
    }

    unsafe {
        (*res_tracker).value = 0;
        (*res_tracker).permissions = permissions;
        set_operation((*res_tracker).operation.as_mut_ptr(), b"none\0");
    }

    match mode {
        1 => unsafe {
            set_operation((*res_tracker).operation.as_mut_ptr(), b"addition\0");
            result = safe_add(value1, value2, permissions);
            (*res_tracker).value = result;

            printf(cstr(b"Mode 1: Addition\n\0"));
            printf(cstr(b"Result: %d\n\0"), result);
        },
        2 => unsafe {
            set_operation((*res_tracker).operation.as_mut_ptr(), b"multiplication\0");
            result = multiply_with_log(value1, value2, &mut log_message);
            (*res_tracker).value = result;

            if log_message.is_null() || strcmp(log_message, cstr(b"\0")) == 0 {
                printf(cstr(b"Log message creation failed\n\0"));
            } else {
                printf(cstr(b"Mode 2: %s\n\0"), log_message);
                free(log_message.cast::<c_void>());
            }
        },
        3 => unsafe {
            set_operation((*res_tracker).operation.as_mut_ptr(), b"array_sum\0");
            let mut values = [value1, value2, value3];
            result = copy_and_sum(values.as_mut_ptr(), 3);
            (*res_tracker).value = result;

            printf(cstr(b"Mode 3: Array Sum\n\0"));
            printf(cstr(b"Result: %d\n\0"), result);
        },
        4 => unsafe {
            set_operation((*res_tracker).operation.as_mut_ptr(), b"complex\0");

            if check_permissions(permissions, 0o100) != 0 {
                result = value1.wrapping_mul(value2).wrapping_add(value3);
            } else {
                result = value1.wrapping_add(value2).wrapping_add(value3);
            }

            (*res_tracker).value = result;
            printf(cstr(b"Mode 4: Complex Calculation\n\0"));
            printf(cstr(b"Result: %d\n\0"), result);
        },
        _ => unsafe {
            printf(cstr(b"Invalid mode\n\0"));
            result = -1;
        },
    }

    unsafe {
        if strcmp((*res_tracker).operation.as_ptr(), cstr(b"none\0")) != 0 {
            printf(
                cstr(b"Operation performed: %s\n\0"),
                (*res_tracker).operation.as_ptr(),
            );
        }

        free(res_tracker.cast::<c_void>());
    }

    result
}
