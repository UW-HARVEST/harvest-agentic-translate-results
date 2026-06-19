use std::ffi::{c_char, c_int};
use std::ptr;

#[repr(C)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

const ADD: &[u8] = b"add\0";
const SUBTRACT: &[u8] = b"subtract\0";
const MULTIPLY: &[u8] = b"multiply\0";
const DIVIDE: &[u8] = b"divide\0";
const UNKNOWN: &[u8] = b"unknown\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    let buffer = unsafe { libc::malloc(size_of::<StringBuffer>()) as *mut StringBuffer };
    if buffer.is_null() {
        return ptr::null_mut();
    }

    let data = unsafe { libc::malloc(initial_capacity as usize) as *mut c_char };
    if data.is_null() {
        unsafe {
            libc::free(buffer.cast());
        }
        return ptr::null_mut();
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
pub unsafe extern "C" fn append_to_buffer(
    buffer: *mut StringBuffer,
    str_: *const c_char,
) -> c_int {
    let str_len = unsafe { libc::strlen(str_) } as c_int;
    let required_capacity = unsafe { (*buffer).length }
        .wrapping_add(str_len)
        .wrapping_add(1);

    if required_capacity > unsafe { (*buffer).capacity } {
        let new_capacity = required_capacity.wrapping_mul(2);
        let new_data = unsafe {
            libc::realloc((*buffer).data.cast(), new_capacity as usize) as *mut c_char
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
        (*buffer).length = (*buffer).length.wrapping_add(str_len);
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    if !buffer.is_null() {
        unsafe {
            if !(*buffer).data.is_null() {
                libc::free((*buffer).data.cast());
            }
            libc::free(buffer.cast());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_name(op_code: c_int) -> *const c_char {
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
    unsafe {
        if libc::strcmp(operation, ADD.as_ptr().cast()) == 0 {
            a.wrapping_add(b)
        } else if libc::strcmp(operation, SUBTRACT.as_ptr().cast()) == 0 {
            a.wrapping_sub(b)
        } else if libc::strcmp(operation, MULTIPLY.as_ptr().cast()) == 0 {
            a.wrapping_mul(b)
        } else if libc::strcmp(operation, DIVIDE.as_ptr().cast()) == 0 {
            if b != 0 {
                a.wrapping_div(b)
            } else {
                0
            }
        } else {
            0
        }
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
    let mut result: c_int = 0;
    let mut temp = [0 as c_char; 64];

    unsafe {
        (*log_buffer).length = 0;

        libc::sprintf(
            temp.as_mut_ptr(),
            c"Starting computation with %d parameters\n".as_ptr(),
            4 as c_int,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        let op1 = get_operation_name(param1 % 4);
        libc::sprintf(
            temp.as_mut_ptr(),
            c"Operation 1: %s(%d, %d)\n".as_ptr(),
            op1,
            param1,
            param2,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        let intermediate1 = perform_operation(param1, param2, op1);
        result = result.wrapping_add(intermediate1);

        let op2 = get_operation_name(param3 % 4);
        libc::sprintf(
            temp.as_mut_ptr(),
            c"Operation 2: %s(%d, %d)\n".as_ptr(),
            op2,
            param3,
            param4,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        let intermediate2 = perform_operation(param3, param4, op2);
        result = result.wrapping_add(intermediate2);

        let op3 = MULTIPLY.as_ptr() as *const c_char;
        libc::sprintf(
            temp.as_mut_ptr(),
            c"Operation 3: %s(%d, %d)\n".as_ptr(),
            op3,
            intermediate1,
            intermediate2,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

        if intermediate3 != 0 {
            result = result.wrapping_div(intermediate3);
        } else {
            result = param1
                .wrapping_add(param2)
                .wrapping_add(param3)
                .wrapping_add(param4);
        }

        libc::sprintf(
            temp.as_mut_ptr(),
            c"Final result: %d\n".as_ptr(),
            result,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        libc::printf(
            c"Computation Log:\n%s\n".as_ptr(),
            (*log_buffer).data,
        );

        destroy_buffer(log_buffer);
    }

    result
}
