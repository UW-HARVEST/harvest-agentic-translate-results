use std::ffi::{c_char, c_int, CStr};
use std::fmt::Write;

#[repr(C)]
pub struct StringBuffer {
    data: *mut c_char,
    capacity: c_int,
    length: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    unsafe {
        let buffer = libc_malloc(std::mem::size_of::<StringBuffer>()) as *mut StringBuffer;
        if buffer.is_null() {
            return std::ptr::null_mut();
        }
        let data = libc_malloc(initial_capacity as usize) as *mut c_char;
        if data.is_null() {
            libc_free(buffer as *mut u8);
            return std::ptr::null_mut();
        }
        (*buffer).data = data;
        (*buffer).capacity = initial_capacity;
        (*buffer).length = 0;
        *data = 0;
        buffer
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn append_to_buffer(buffer: *mut StringBuffer, str_ptr: *const c_char) -> c_int {
    unsafe {
        let s = CStr::from_ptr(str_ptr);
        let str_len = s.to_bytes().len() as c_int;
        let required_capacity = (*buffer).length + str_len + 1;

        if required_capacity > (*buffer).capacity {
            let new_capacity = required_capacity * 2;
            let new_data = libc_realloc((*buffer).data as *mut u8, new_capacity as usize) as *mut c_char;
            if new_data.is_null() {
                return -1;
            }
            (*buffer).data = new_data;
            (*buffer).capacity = new_capacity;
        }

        std::ptr::copy_nonoverlapping(
            str_ptr as *const u8,
            ((*buffer).data as *mut u8).add((*buffer).length as usize),
            (str_len + 1) as usize,
        );
        (*buffer).length += str_len;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    unsafe {
        if !buffer.is_null() {
            if !(*buffer).data.is_null() {
                libc_free((*buffer).data as *mut u8);
            }
            libc_free(buffer as *mut u8);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_name(op_code: c_int) -> *const c_char {
    match op_code {
        0 => c"add".as_ptr(),
        1 => c"subtract".as_ptr(),
        2 => c"multiply".as_ptr(),
        3 => c"divide".as_ptr(),
        _ => c"unknown".as_ptr(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn perform_operation(a: c_int, b: c_int, operation: *const c_char) -> c_int {
    unsafe {
        let op = CStr::from_ptr(operation);
        match op.to_bytes() {
            b"add" => a.wrapping_add(b),
            b"subtract" => a.wrapping_sub(b),
            b"multiply" => a.wrapping_mul(b),
            b"divide" => {
                if b != 0 { a.wrapping_div(b) } else { 0 }
            }
            _ => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn buffapp(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut log_buffer = String::with_capacity(32);
    let mut result: c_int;

    let _ = write!(log_buffer, "Starting computation with {} parameters\n", 4);

    let op1 = get_operation_name_str(param1 % 4);
    let _ = write!(log_buffer, "Operation 1: {}({}, {})\n", op1, param1, param2);

    let intermediate1 = perform_operation_internal(param1, param2, op1);
    result = intermediate1;

    let op2 = get_operation_name_str(param3 % 4);
    let _ = write!(log_buffer, "Operation 2: {}({}, {})\n", op2, param3, param4);

    let intermediate2 = perform_operation_internal(param3, param4, op2);
    result = result.wrapping_add(intermediate2);

    let op3 = "multiply";
    let _ = write!(
        log_buffer,
        "Operation 3: {}({}, {})\n",
        op3, intermediate1, intermediate2
    );

    let intermediate3 = perform_operation_internal(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result = result.wrapping_div(intermediate3);
    } else {
        result = param1
            .wrapping_add(param2)
            .wrapping_add(param3)
            .wrapping_add(param4);
    }

    let _ = write!(log_buffer, "Final result: {}\n", result);

    print!("Computation Log:\n{}\n", log_buffer);

    result
}

// Internal helpers (not exported)
fn get_operation_name_str(op_code: c_int) -> &'static str {
    match op_code {
        0 => "add",
        1 => "subtract",
        2 => "multiply",
        3 => "divide",
        _ => "unknown",
    }
}

fn perform_operation_internal(a: c_int, b: c_int, operation: &str) -> c_int {
    match operation {
        "add" => a.wrapping_add(b),
        "subtract" => a.wrapping_sub(b),
        "multiply" => a.wrapping_mul(b),
        "divide" => {
            if b != 0 { a.wrapping_div(b) } else { 0 }
        }
        _ => 0,
    }
}

// Minimal libc wrappers
unsafe fn libc_malloc(size: usize) -> *mut u8 {
    extern "C" { fn malloc(size: usize) -> *mut u8; }
    unsafe { malloc(size) }
}

unsafe fn libc_realloc(ptr: *mut u8, size: usize) -> *mut u8 {
    extern "C" { fn realloc(ptr: *mut u8, size: usize) -> *mut u8; }
    unsafe { realloc(ptr, size) }
}

unsafe fn libc_free(ptr: *mut u8) {
    extern "C" { fn free(ptr: *mut u8); }
    unsafe { free(ptr) }
}
