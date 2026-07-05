





use std::ffi::CString;

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Result_0 {
    pub value: ::core::ffi::c_int,
    pub operation: [::core::ffi::c_char; 32],
    pub permissions: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const READ_PERM: ::core::ffi::c_int = 0o400 as ::core::ffi::c_int;
pub const WRITE_PERM: ::core::ffi::c_int = 0o200 as ::core::ffi::c_int;
#[no_mangle]
pub fn create_result_string(op: &str, val: i32) -> *mut ::core::ffi::c_char {
    let sanitized = op.replace('\0', "");
    match CString::new(format!("Operation: {}, Value: {}", sanitized, val)) {
        Ok(s) => s.into_raw(),
        Err(_) => ::core::ptr::null_mut(),
    }
}

#[no_mangle]
pub fn check_permissions(perms: i32, required: i32) -> i32 {
    ((perms & required) == required) as i32
}

#[no_mangle]
pub fn safe_add(a: i32, b: i32, perms: i32) -> i32 {
    let has_permissions = check_permissions(perms, READ_PERM | WRITE_PERM) != 0;
    if !has_permissions {
        eprintln!("Insufficient permissions for addition");
        return 0;
    }
    a + b
}

#[no_mangle]
pub fn multiply_with_log(a: i32, b: i32, log_msg: &mut *mut ::core::ffi::c_char) -> i32 {
    let product = a * b;
    *log_msg = create_result_string("multiply", product);
    if (*log_msg).is_null() {
        0
    } else {
        product
    }
}

#[no_mangle]
pub fn copy_and_sum(src: &[i32], count: i32) -> i32 {
    if count < 0 {
        eprintln!("Invalid count");
        return -1;
    }

    let count = count as usize;
    if src.len() < count {
        eprintln!("Source slice is too small");
        return -1;
    }

    let dest = src[..count].to_vec();
    dest.into_iter().sum()
}

#[no_mangle]
pub fn compare_operations(op1: Option<&str>, op2: Option<&str>) -> i32 {
    match (op1, op2) {
        (Some(op1), Some(op2)) => {
            use std::cmp::Ordering;
            match op1.cmp(op2) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }
        }
        _ => {
            println!("One or both operation strings are NULL");
            -1
        }
    }
}

#[no_mangle]
pub fn complexmode(mode: i32, value1: i32, value2: i32, value3: i32) -> i32 {
    fn set_operation(operation: &mut [i8], value: &str) {
        operation.fill(0);
        for (dst, src) in operation.iter_mut().zip(value.bytes()) {
            *dst = src as i8;
        }
    }

    fn get_operation(operation: &[i8]) -> String {
        let len = operation.iter().position(|&b| b == 0).unwrap_or(operation.len());
        operation[..len].iter().map(|&b| b as u8 as char).collect()
    }

    let mut result = 0;
    let permissions = 0o644;

    let mut res_tracker = Result_0 {
        value: 0,
        permissions,
        operation: [0; 32],
    };
    set_operation(&mut res_tracker.operation, "none");

    match mode {
        1 => {
            set_operation(&mut res_tracker.operation, "addition");
            result = safe_add(value1, value2, permissions);
            res_tracker.value = result;
            println!("Mode 1: Addition");
            println!("Result: {}", result);
        }
        2 => {
            set_operation(&mut res_tracker.operation, "multiplication");
            let mut log_message = std::ptr::null_mut();
            result = multiply_with_log(value1, value2, &mut log_message);
            res_tracker.value = result;

            if log_message.is_null() {
                println!("Log message creation failed");
            } else {
                println!("Mode 2: {:?}", log_message);
            }
        }
        3 => {
            set_operation(&mut res_tracker.operation, "array_sum");
            let values = [value1, value2, value3];
            result = copy_and_sum(&values, 3);
            res_tracker.value = result;
            println!("Mode 3: Array Sum");
            println!("Result: {}", result);
        }
        4 => {
            set_operation(&mut res_tracker.operation, "complex");
            if unsafe { check_permissions(permissions, 0o100) } != 0 {
                result = value1 * value2 + value3;
            } else {
                result = value1 + value2 + value3;
            }
            res_tracker.value = result;
            println!("Mode 4: Complex Calculation");
            println!("Result: {}", result);
        }
        _ => {
            println!("Invalid mode");
            result = -1;
        }
    }

    let operation = get_operation(&res_tracker.operation);
    if operation != "none" {
        println!("Operation performed: {}", operation);
    }

    result
}

