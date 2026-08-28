use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

#[repr(C)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn sprintf(dest: *mut c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

const ADD: &[u8] = b"add\0";
const SUBTRACT: &[u8] = b"subtract\0";
const MULTIPLY: &[u8] = b"multiply\0";
const DIVIDE: &[u8] = b"divide\0";
const UNKNOWN: &[u8] = b"unknown\0";

#[inline]
fn c_ptr(value: &'static [u8]) -> *const c_char {
    value.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    let buffer = unsafe { malloc(size_of::<StringBuffer>()) }.cast::<StringBuffer>();
    if buffer.is_null() {
        return ptr::null_mut();
    }

    let data = unsafe { malloc(initial_capacity as usize) }.cast::<c_char>();
    if data.is_null() {
        unsafe { free(buffer.cast()) };
        return ptr::null_mut();
    }

    unsafe {
        (*buffer).data = data;
        (*buffer).capacity = initial_capacity;
        (*buffer).length = 0;
        *data = 0;
    }

    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn append_to_buffer(
    buffer: *mut StringBuffer,
    string: *const c_char,
) -> c_int {
    let string_length = unsafe { strlen(string) } as c_int;
    let required_capacity = unsafe { (*buffer).length } + string_length + 1;

    if required_capacity > unsafe { (*buffer).capacity } {
        let new_capacity = required_capacity * 2;
        let new_data =
            unsafe { realloc((*buffer).data.cast(), new_capacity as usize) }.cast::<c_char>();

        if new_data.is_null() {
            return -1;
        }

        unsafe {
            (*buffer).data = new_data;
            (*buffer).capacity = new_capacity;
        }
    }

    unsafe {
        strcpy((*buffer).data.offset((*buffer).length as isize), string);
        (*buffer).length += string_length;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    if !buffer.is_null() {
        if !unsafe { (*buffer).data }.is_null() {
            unsafe { free((*buffer).data.cast()) };
        }
        unsafe { free(buffer.cast()) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_name(operation_code: c_int) -> *const c_char {
    c_ptr(match operation_code {
        0 => ADD,
        1 => SUBTRACT,
        2 => MULTIPLY,
        3 => DIVIDE,
        _ => UNKNOWN,
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(
    first: c_int,
    second: c_int,
    operation: *const c_char,
) -> c_int {
    if unsafe { strcmp(operation, c_ptr(ADD)) } == 0 {
        first + second
    } else if unsafe { strcmp(operation, c_ptr(SUBTRACT)) } == 0 {
        first - second
    } else if unsafe { strcmp(operation, c_ptr(MULTIPLY)) } == 0 {
        first * second
    } else if unsafe { strcmp(operation, c_ptr(DIVIDE)) } == 0 {
        if second != 0 { first / second } else { 0 }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffapp(
    parameter1: c_int,
    parameter2: c_int,
    parameter3: c_int,
    parameter4: c_int,
) -> c_int {
    let log_buffer = unsafe { create_buffer(32) };
    let mut result = 0;
    let mut temporary = [0 as c_char; 64];

    unsafe { (*log_buffer).length = 0 };

    unsafe {
        sprintf(
            temporary.as_mut_ptr(),
            c_ptr(b"Starting computation with %d parameters\n\0"),
            4 as c_int,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());
    }

    let operation1 = get_operation_name(parameter1 % 4);
    unsafe {
        sprintf(
            temporary.as_mut_ptr(),
            c_ptr(b"Operation 1: %s(%d, %d)\n\0"),
            operation1,
            parameter1,
            parameter2,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());
    }

    let intermediate1 = unsafe { perform_operation(parameter1, parameter2, operation1) };
    result += intermediate1;

    let operation2 = get_operation_name(parameter3 % 4);
    unsafe {
        sprintf(
            temporary.as_mut_ptr(),
            c_ptr(b"Operation 2: %s(%d, %d)\n\0"),
            operation2,
            parameter3,
            parameter4,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());
    }

    let intermediate2 = unsafe { perform_operation(parameter3, parameter4, operation2) };
    result += intermediate2;

    let operation3 = c_ptr(MULTIPLY);
    unsafe {
        sprintf(
            temporary.as_mut_ptr(),
            c_ptr(b"Operation 3: %s(%d, %d)\n\0"),
            operation3,
            intermediate1,
            intermediate2,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());
    }

    let intermediate3 = unsafe { perform_operation(intermediate1, intermediate2, operation3) };

    if intermediate3 != 0 {
        result /= intermediate3;
    } else {
        result = parameter1 + parameter2 + parameter3 + parameter4;
    }

    unsafe {
        sprintf(
            temporary.as_mut_ptr(),
            c_ptr(b"Final result: %d\n\0"),
            result,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());

        printf(c_ptr(b"Computation Log:\n%s\n\0"), (*log_buffer).data);

        destroy_buffer(log_buffer);
    }

    result
}
