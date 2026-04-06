use std::ffi::{c_char, c_int, CStr};

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
    let op = unsafe { CStr::from_ptr(operation) }.to_bytes();
    match op {
        b"add" => a.wrapping_add(b),
        b"subtract" => a.wrapping_sub(b),
        b"multiply" => a.wrapping_mul(b),
        b"divide" => {
            if b != 0 { a.wrapping_div(b) } else { 0 }
        }
        _ => 0,
    }
}

#[repr(C)]
pub struct StringBuffer {
    data: *mut c_char,
    capacity: c_int,
    length: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    let data = unsafe { libc::malloc(initial_capacity as usize) as *mut c_char };
    if data.is_null() {
        return std::ptr::null_mut();
    }
    let buffer = unsafe { libc::malloc(std::mem::size_of::<StringBuffer>()) as *mut StringBuffer };
    if buffer.is_null() {
        unsafe { libc::free(data as *mut _) };
        return std::ptr::null_mut();
    }
    unsafe {
        (*buffer).data = data;
        (*buffer).capacity = initial_capacity;
        (*buffer).length = 0;
        *data = 0; // null terminator
    }
    buffer
}

#[unsafe(no_mangle)]
pub extern "C" fn append_to_buffer(buffer: *mut StringBuffer, str_ptr: *const c_char) -> c_int {
    let str_len = unsafe { libc::strlen(str_ptr as *const _) } as c_int;
    let required_capacity = unsafe { (*buffer).length + str_len + 1 };

    if required_capacity > unsafe { (*buffer).capacity } {
        let new_capacity = required_capacity * 2;
        let new_data = unsafe { libc::realloc((*buffer).data as *mut _, new_capacity as usize) as *mut c_char };
        if new_data.is_null() {
            return -1;
        }
        unsafe {
            (*buffer).data = new_data;
            (*buffer).capacity = new_capacity;
        }
    }

    unsafe {
        libc::strcpy((*buffer).data.offset((*buffer).length as isize) as *mut _, str_ptr as *const _);
        (*buffer).length += str_len;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    if !buffer.is_null() {
        unsafe {
            if !(*buffer).data.is_null() {
                libc::free((*buffer).data as *mut _);
            }
            libc::free(buffer as *mut _);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn buffapp(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let log_buffer = create_buffer(32);
    let mut result: c_int = 0;
    let mut temp = [0u8; 64];

    unsafe { (*log_buffer).length = 0 };

    let _ = fmt_to_buf(&mut temp, format_args!("Starting computation with {} parameters\n", 4));
    append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

    let op1 = get_operation_name(param1 % 4);
    let op1_str = unsafe { CStr::from_ptr(op1) };
    let _ = fmt_to_buf(&mut temp, format_args!("Operation 1: {}({}, {})\n", op1_str.to_str().unwrap(), param1, param2));
    append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

    let intermediate1 = perform_operation(param1, param2, op1);
    result += intermediate1;

    let op2 = get_operation_name(param3 % 4);
    let op2_str = unsafe { CStr::from_ptr(op2) };
    let _ = fmt_to_buf(&mut temp, format_args!("Operation 2: {}({}, {})\n", op2_str.to_str().unwrap(), param3, param4));
    append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

    let intermediate2 = perform_operation(param3, param4, op2);
    result += intermediate2;

    let op3 = c"multiply".as_ptr();
    let op3_str = "multiply";
    let _ = fmt_to_buf(&mut temp, format_args!("Operation 3: {}({}, {})\n", op3_str, intermediate1, intermediate2));
    append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result = result / intermediate3;
    } else {
        result = param1 + param2 + param3 + param4;
    }

    let _ = fmt_to_buf(&mut temp, format_args!("Final result: {}\n", result));
    append_to_buffer(log_buffer, temp.as_ptr() as *const c_char);

    unsafe {
        libc::printf(c"Computation Log:\n%s\n".as_ptr(), (*log_buffer).data);
    }

    destroy_buffer(log_buffer);

    result
}

fn fmt_to_buf(buf: &mut [u8], args: std::fmt::Arguments) -> usize {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(&mut buf[..]);
    let _ = write!(cursor, "{}", args);
    let pos = cursor.position() as usize;
    buf[pos] = 0; // null terminate
    pos
}
