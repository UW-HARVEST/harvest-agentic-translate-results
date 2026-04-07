use std::ffi::{c_char, c_int};
use std::ptr;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    if (perms & required) == required { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let buf = unsafe { libc::malloc(64) as *mut c_char };
    if buf.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        libc::snprintf(
            buf,
            64,
            b"Operation: %s, Value: %d\0".as_ptr() as *const c_char,
            op,
            val,
        );
    }
    buf
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        unsafe { libc::printf(b"Insufficient permissions for addition\n\0".as_ptr() as *const c_char) };
        return 0;
    }
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_log(a: c_int, b: c_int, log_msg: *mut *mut c_char) -> c_int {
    unsafe {
        *log_msg = create_result_string(b"multiply\0".as_ptr() as *const c_char, a * b);
        if (*log_msg).is_null() {
            return 0;
        }
    }
    a * b
}

#[unsafe(no_mangle)]
pub extern "C" fn copy_and_sum(src: *const c_int, count: c_int) -> c_int {
    if src.is_null() {
        unsafe { libc::printf(b"Source pointer is NULL\n\0".as_ptr() as *const c_char) };
        return -1;
    }
    let count_usize = count as usize;
    let dest = unsafe { libc::malloc(count_usize * std::mem::size_of::<c_int>()) as *mut c_int };
    if dest.is_null() {
        unsafe { libc::printf(b"Memory allocation failed\n\0".as_ptr() as *const c_char) };
        return -1;
    }
    unsafe {
        libc::memcpy(
            dest as *mut libc::c_void,
            src as *const libc::c_void,
            count_usize * std::mem::size_of::<c_int>(),
        );
    }
    let mut sum: c_int = 0;
    for i in 0..count_usize {
        sum += unsafe { *dest.add(i) };
    }
    unsafe { libc::free(dest as *mut libc::c_void) };
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        unsafe { libc::printf(b"One or both operation strings are NULL\n\0".as_ptr() as *const c_char) };
        return -1;
    }
    unsafe { libc::strcmp(op1, op2) }
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut log_message: *mut c_char = ptr::null_mut();
    let permissions: c_int = 0o644;

    let res_tracker = unsafe { libc::malloc(std::mem::size_of::<Result>()) as *mut Result };
    if res_tracker.is_null() {
        unsafe { libc::printf(b"Failed to allocate result tracker\n\0".as_ptr() as *const c_char) };
        return -1;
    }

    unsafe {
        (*res_tracker).value = 0;
        (*res_tracker).permissions = permissions;
        libc::strcpy((*res_tracker).operation.as_mut_ptr(), b"none\0".as_ptr() as *const c_char);
    }

    match mode {
        1 => unsafe {
            libc::strcpy((*res_tracker).operation.as_mut_ptr(), b"addition\0".as_ptr() as *const c_char);
            result = safe_add(value1, value2, permissions);
            (*res_tracker).value = result;
            libc::printf(b"Mode 1: Addition\n\0".as_ptr() as *const c_char);
            libc::printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
        },
        2 => unsafe {
            libc::strcpy((*res_tracker).operation.as_mut_ptr(), b"multiplication\0".as_ptr() as *const c_char);
            result = multiply_with_log(value1, value2, &mut log_message);
            (*res_tracker).value = result;
            if log_message.is_null() || libc::strcmp(log_message, b"\0".as_ptr() as *const c_char) == 0 {
                libc::printf(b"Log message creation failed\n\0".as_ptr() as *const c_char);
            } else {
                libc::printf(b"Mode 2: %s\n\0".as_ptr() as *const c_char, log_message);
                libc::free(log_message as *mut libc::c_void);
            }
        },
        3 => unsafe {
            libc::strcpy((*res_tracker).operation.as_mut_ptr(), b"array_sum\0".as_ptr() as *const c_char);
            let values: [c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(values.as_ptr(), 3);
            (*res_tracker).value = result;
            libc::printf(b"Mode 3: Array Sum\n\0".as_ptr() as *const c_char);
            libc::printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
        },
        4 => unsafe {
            libc::strcpy((*res_tracker).operation.as_mut_ptr(), b"complex\0".as_ptr() as *const c_char);
            if check_permissions(permissions, 0o100) != 0 {
                result = (value1 * value2) + value3;
            } else {
                result = value1 + value2 + value3;
            }
            (*res_tracker).value = result;
            libc::printf(b"Mode 4: Complex Calculation\n\0".as_ptr() as *const c_char);
            libc::printf(b"Result: %d\n\0".as_ptr() as *const c_char, result);
        },
        _ => {
            unsafe { libc::printf(b"Invalid mode\n\0".as_ptr() as *const c_char) };
            result = -1;
        }
    }

    unsafe {
        if libc::strcmp((*res_tracker).operation.as_ptr(), b"none\0".as_ptr() as *const c_char) != 0 {
            libc::printf(b"Operation performed: %s\n\0".as_ptr() as *const c_char, (*res_tracker).operation.as_ptr());
        }
        libc::free(res_tracker as *mut libc::c_void);
    }

    result
}

#[repr(C)]
struct Result {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}
