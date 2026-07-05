




use std::ffi::CStr;
use std::ffi::CString;

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn sprintf(
        __s: *mut ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct StringBuffer {
    pub data: *mut ::core::ffi::c_char,
    pub capacity: ::core::ffi::c_int,
    pub length: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn create_buffer(initial_capacity: i32) -> *mut StringBuffer {
    let capacity = if initial_capacity <= 0 { 1 } else { initial_capacity } as usize;

    let data = vec![0 as ::core::ffi::c_char; capacity].into_boxed_slice();
    let data_ptr = Box::into_raw(data) as *mut ::core::ffi::c_char;

    let buffer = Box::new(StringBuffer {
        data: data_ptr,
        capacity: capacity as ::core::ffi::c_int,
        length: 0,
    });

    Box::into_raw(buffer)
}

#[no_mangle]
pub fn append_to_buffer(
    buffer: &mut StringBuffer,
    str: &CStr,
) -> ::core::ffi::c_int {
    let s = match str.to_str() {
        Ok(s) => s,
        Err(_) => return -(1 as ::core::ffi::c_int),
    };

    let current = if buffer.data.is_null() {
        String::new()
    } else {
        let existing = unsafe { CStr::from_ptr(buffer.data) };
        match existing.to_str() {
            Ok(s) => s.to_owned(),
            Err(_) => return -(1 as ::core::ffi::c_int),
        }
    };

    let mut combined = current;
    combined.push_str(s);

    let c_string = match CString::new(combined) {
        Ok(c) => c,
        Err(_) => return -(1 as ::core::ffi::c_int),
    };

    let len = c_string.as_bytes().len() as ::core::ffi::c_int;
    let cap = c_string.as_bytes_with_nul().len() as ::core::ffi::c_int;
    let raw = c_string.into_raw();

    buffer.data = raw;
    buffer.length = len;
    buffer.capacity = cap;

    0 as ::core::ffi::c_int
}

#[no_mangle]
pub fn destroy_buffer(buffer: *mut StringBuffer) {
    if buffer.is_null() {
        return;
    }

    let _ = buffer;
}

#[no_mangle]
pub fn get_operation_name(op_code: ::core::ffi::c_int) -> &'static ::std::ffi::CStr {
    match op_code {
        0 => ::std::ffi::CStr::from_bytes_with_nul(b"add\0").unwrap(),
        1 => ::std::ffi::CStr::from_bytes_with_nul(b"subtract\0").unwrap(),
        2 => ::std::ffi::CStr::from_bytes_with_nul(b"multiply\0").unwrap(),
        3 => ::std::ffi::CStr::from_bytes_with_nul(b"divide\0").unwrap(),
        _ => ::std::ffi::CStr::from_bytes_with_nul(b"unknown\0").unwrap(),
    }
}

#[no_mangle]
pub fn perform_operation(a: i32, b: i32, operation: &str) -> i32 {
    match operation {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => {
            if b != 0 {
                a / b
            } else {
                0
            }
        }
        _ => 0,
    }
}

#[no_mangle]
pub fn buffapp(
    param1: ::core::ffi::c_int,
    param2: ::core::ffi::c_int,
    param3: ::core::ffi::c_int,
    param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0;
    let mut log = String::new();

    log.push_str("Starting computation with 4 parameters\n");

    let op1 = get_operation_name(param1 % 4);
    let op1_str = op1.to_string_lossy();
    log.push_str(&format!("Operation 1: {}({}, {})\n", op1_str, param1, param2));
    let intermediate1 = perform_operation(param1, param2, &op1_str);
    result += intermediate1;

    let op2 = get_operation_name(param3 % 4);
    let op2_str = op2.to_string_lossy();
    log.push_str(&format!("Operation 2: {}({}, {})\n", op2_str, param3, param4));
    let intermediate2 = perform_operation(param3, param4, &op2_str);
    result += intermediate2;

    let op3 = "multiply";
    log.push_str(&format!(
        "Operation 3: {}({}, {})\n",
        op3, intermediate1, intermediate2
    ));
    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result /= intermediate3;
    } else {
        result = param1 + param2 + param3 + param4;
    }

    log.push_str(&format!("Final result: {}\n", result));
    println!("Computation Log:\n{}", log);

    result
}

