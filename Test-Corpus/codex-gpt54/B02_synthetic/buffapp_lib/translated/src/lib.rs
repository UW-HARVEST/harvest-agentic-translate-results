use libc::{c_char, c_int, c_void};
use std::mem::size_of;

#[repr(C)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

static ADD: &[u8] = b"add\0";
static SUBTRACT: &[u8] = b"subtract\0";
static MULTIPLY: &[u8] = b"multiply\0";
static DIVIDE: &[u8] = b"divide\0";
static UNKNOWN: &[u8] = b"unknown\0";
static STARTING_COMPUTATION_FMT: &[u8] = b"Starting computation with %d parameters\n\0";
static OPERATION_FMT: &[u8] = b"Operation %d: %s(%d, %d)\n\0";
static FINAL_RESULT_FMT: &[u8] = b"Final result: %d\n\0";
static COMPUTATION_LOG_FMT: &[u8] = b"Computation Log:\n%s\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    let buffer = unsafe { libc::malloc(size_of::<StringBuffer>()) as *mut StringBuffer };
    if buffer.is_null() {
        return std::ptr::null_mut();
    }

    let data = unsafe { libc::malloc(initial_capacity as usize) as *mut c_char };
    if data.is_null() {
        unsafe { libc::free(buffer.cast::<c_void>()) };
        return std::ptr::null_mut();
    }

    unsafe {
        (*buffer).data = data;
        (*buffer).capacity = initial_capacity;
        (*buffer).length = 0;
        *(*buffer).data = 0;
    }

    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn append_to_buffer(buffer: *mut StringBuffer, str_: *const c_char) -> c_int {
    let str_len = unsafe { libc::strlen(str_) as c_int };
    let required_capacity = unsafe { (*buffer).length + str_len + 1 };

    if required_capacity > unsafe { (*buffer).capacity } {
        let new_capacity = required_capacity * 2;
        let new_data = unsafe {
            libc::realloc((*buffer).data.cast::<c_void>(), new_capacity as usize) as *mut c_char
        };

        if new_data.is_null() {
            return -1;
        }

        unsafe {
            (*buffer).data = new_data;
            (*buffer).capacity = new_capacity;
        }
    }

    unsafe {
        libc::strcpy((*buffer).data.add((*buffer).length as usize), str_);
        (*buffer).length += str_len;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    if !buffer.is_null() {
        unsafe {
            if !(*buffer).data.is_null() {
                libc::free((*buffer).data.cast::<c_void>());
            }
            libc::free(buffer.cast::<c_void>());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_operation_name(op_code: c_int) -> *const c_char {
    match op_code {
        0 => ADD.as_ptr().cast(),
        1 => SUBTRACT.as_ptr().cast(),
        2 => MULTIPLY.as_ptr().cast(),
        3 => DIVIDE.as_ptr().cast(),
        _ => UNKNOWN.as_ptr().cast(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(
    a: c_int,
    b: c_int,
    operation: *const c_char,
) -> c_int {
    if unsafe { libc::strcmp(operation, ADD.as_ptr().cast()) } == 0 {
        a + b
    } else if unsafe { libc::strcmp(operation, SUBTRACT.as_ptr().cast()) } == 0 {
        a - b
    } else if unsafe { libc::strcmp(operation, MULTIPLY.as_ptr().cast()) } == 0 {
        a * b
    } else if unsafe { libc::strcmp(operation, DIVIDE.as_ptr().cast()) } == 0 {
        if b != 0 { a / b } else { 0 }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffapp(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let log_buffer = unsafe { create_buffer(32) };
    let mut result = 0;
    let mut temp = [0 as c_char; 64];

    unsafe {
        (*log_buffer).length = 0;
    }

    unsafe {
        libc::sprintf(
            temp.as_mut_ptr(),
            STARTING_COMPUTATION_FMT.as_ptr().cast(),
            4,
        );
        append_to_buffer(log_buffer, temp.as_ptr());
    }

    let op1 = unsafe { get_operation_name(param1 % 4) };
    unsafe {
        libc::sprintf(
            temp.as_mut_ptr(),
            OPERATION_FMT.as_ptr().cast(),
            1,
            op1,
            param1,
            param2,
        );
        append_to_buffer(log_buffer, temp.as_ptr());
    }

    let intermediate1 = unsafe { perform_operation(param1, param2, op1) };
    result += intermediate1;

    let op2 = unsafe { get_operation_name(param3 % 4) };
    unsafe {
        libc::sprintf(
            temp.as_mut_ptr(),
            OPERATION_FMT.as_ptr().cast(),
            2,
            op2,
            param3,
            param4,
        );
        append_to_buffer(log_buffer, temp.as_ptr());
    }

    let intermediate2 = unsafe { perform_operation(param3, param4, op2) };
    result += intermediate2;

    let op3 = MULTIPLY.as_ptr().cast();
    unsafe {
        libc::sprintf(
            temp.as_mut_ptr(),
            OPERATION_FMT.as_ptr().cast(),
            3,
            op3,
            intermediate1,
            intermediate2,
        );
        append_to_buffer(log_buffer, temp.as_ptr());
    }

    let intermediate3 = unsafe { perform_operation(intermediate1, intermediate2, op3) };

    if intermediate3 != 0 {
        result /= intermediate3;
    } else {
        result = param1 + param2 + param3 + param4;
    }

    unsafe {
        libc::sprintf(temp.as_mut_ptr(), FINAL_RESULT_FMT.as_ptr().cast(), result);
        append_to_buffer(log_buffer, temp.as_ptr());
        libc::printf(COMPUTATION_LOG_FMT.as_ptr().cast(), (*log_buffer).data);
        destroy_buffer(log_buffer);
    }

    result
}
