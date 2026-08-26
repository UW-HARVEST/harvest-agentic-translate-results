use libc::{c_char, c_int, c_void, size_t};
use std::ptr;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;

#[repr(C)]
struct ResultTracker {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

const OP_NONE: &[u8] = b"none\0";
const OP_MULTIPLY: &[u8] = b"multiply\0";
const OP_ADDITION: &[u8] = b"addition\0";
const OP_MULTIPLICATION: &[u8] = b"multiplication\0";
const OP_ARRAY_SUM: &[u8] = b"array_sum\0";
const OP_COMPLEX: &[u8] = b"complex\0";

const MSG_INSUFFICIENT_PERMS: &[u8] = b"Insufficient permissions for addition\n\0";
const MSG_NULL_SOURCE: &[u8] = b"Source pointer is NULL\n\0";
const MSG_ALLOC_FAILED: &[u8] = b"Memory allocation failed\n\0";
const MSG_NULL_OPS: &[u8] = b"One or both operation strings are NULL\n\0";
const MSG_TRACKER_ALLOC_FAILED: &[u8] = b"Failed to allocate result tracker\n\0";
const MSG_MODE1: &[u8] = b"Mode 1: Addition\n\0";
const MSG_MODE3: &[u8] = b"Mode 3: Array Sum\n\0";
const MSG_MODE4: &[u8] = b"Mode 4: Complex Calculation\n\0";
const MSG_INVALID_MODE: &[u8] = b"Invalid mode\n\0";
const MSG_LOG_FAILED: &[u8] = b"Log message creation failed\n\0";
const FMT_RESULT: &[u8] = b"Result: %d\n\0";
const FMT_MODE2: &[u8] = b"Mode 2: %s\n\0";
const FMT_OPERATION_PERFORMED: &[u8] = b"Operation performed: %s\n\0";
const FMT_RESULT_STRING: &[u8] = b"Operation: %s, Value: %d\0";

unsafe fn printf1(format: &[u8], arg1: c_int) {
    unsafe {
        libc::printf(format.as_ptr().cast(), arg1);
    }
}

unsafe fn printf_str(format: &[u8], arg1: *const c_char) {
    unsafe {
        libc::printf(format.as_ptr().cast(), arg1);
    }
}

unsafe fn puts_bytes(message: &[u8]) {
    unsafe {
        libc::printf(message.as_ptr().cast());
    }
}

unsafe fn copy_c_string_fixed(dst: *mut c_char, src: &[u8]) {
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr().cast::<c_char>(), dst, src.len());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let str_ptr = unsafe { libc::malloc(64usize) as *mut c_char };
    if str_ptr.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        libc::snprintf(
            str_ptr,
            64usize,
            FMT_RESULT_STRING.as_ptr().cast(),
            op,
            val,
        );
    }
    str_ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    ((perms & required) == required) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if unsafe { check_permissions(perms, READ_PERM | WRITE_PERM) } == 0 {
        unsafe {
            puts_bytes(MSG_INSUFFICIENT_PERMS);
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
        *log_msg = create_result_string(OP_MULTIPLY.as_ptr().cast(), product);
    }
    if unsafe { (*log_msg).is_null() } {
        return 0;
    }
    product
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        unsafe {
            puts_bytes(MSG_NULL_SOURCE);
        }
        return -1;
    }

    let size = (count as isize as usize).wrapping_mul(std::mem::size_of::<c_int>());
    let dest = unsafe { libc::malloc(size as size_t) as *mut c_int };
    if dest.is_null() {
        unsafe {
            puts_bytes(MSG_ALLOC_FAILED);
        }
        return -1;
    }

    unsafe {
        libc::memcpy(dest.cast::<c_void>(), src.cast::<c_void>(), size as size_t);
    }

    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(unsafe { *dest.add(i as usize) });
        i += 1;
    }

    unsafe {
        libc::free(dest.cast::<c_void>());
    }
    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        unsafe {
            puts_bytes(MSG_NULL_OPS);
        }
        return -1;
    }

    unsafe { libc::strcmp(op1, op2) }
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

    let res_tracker =
        unsafe { libc::malloc(std::mem::size_of::<ResultTracker>() as size_t) as *mut ResultTracker };
    if res_tracker.is_null() {
        unsafe {
            puts_bytes(MSG_TRACKER_ALLOC_FAILED);
        }
        return -1;
    }

    unsafe {
        (*res_tracker).value = 0;
        (*res_tracker).permissions = permissions;
        copy_c_string_fixed((*res_tracker).operation.as_mut_ptr(), OP_NONE);
    }

    match mode {
        1 => {
            unsafe {
                copy_c_string_fixed((*res_tracker).operation.as_mut_ptr(), OP_ADDITION);
                result = safe_add(value1, value2, permissions);
                (*res_tracker).value = result;

                puts_bytes(MSG_MODE1);
                printf1(FMT_RESULT, result);
            }
        }
        2 => {
            unsafe {
                copy_c_string_fixed((*res_tracker).operation.as_mut_ptr(), OP_MULTIPLICATION);
                result = multiply_with_log(value1, value2, &mut log_message);
                (*res_tracker).value = result;

                if log_message.is_null() || libc::strcmp(log_message, c"".as_ptr()) == 0 {
                    puts_bytes(MSG_LOG_FAILED);
                } else {
                    printf_str(FMT_MODE2, log_message);
                    libc::free(log_message.cast::<c_void>());
                }
            }
        }
        3 => {
            let values = [value1, value2, value3];
            unsafe {
                copy_c_string_fixed((*res_tracker).operation.as_mut_ptr(), OP_ARRAY_SUM);
                result = copy_and_sum(values.as_ptr() as *mut c_int, 3);
                (*res_tracker).value = result;

                puts_bytes(MSG_MODE3);
                printf1(FMT_RESULT, result);
            }
        }
        4 => {
            unsafe {
                copy_c_string_fixed((*res_tracker).operation.as_mut_ptr(), OP_COMPLEX);

                if check_permissions(permissions, 0o100) != 0 {
                    result = value1.wrapping_mul(value2).wrapping_add(value3);
                } else {
                    result = value1.wrapping_add(value2).wrapping_add(value3);
                }

                (*res_tracker).value = result;
                puts_bytes(MSG_MODE4);
                printf1(FMT_RESULT, result);
            }
        }
        _ => {
            unsafe {
                puts_bytes(MSG_INVALID_MODE);
            }
            result = -1;
        }
    }

    unsafe {
        if libc::strcmp((*res_tracker).operation.as_ptr(), OP_NONE.as_ptr().cast()) != 0 {
            printf_str(FMT_OPERATION_PERFORMED, (*res_tracker).operation.as_ptr());
        }

        libc::free(res_tracker.cast::<c_void>());
    }

    result
}
