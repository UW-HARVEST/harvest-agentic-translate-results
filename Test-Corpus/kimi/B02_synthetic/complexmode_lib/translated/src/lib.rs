use std::ffi::{c_char, c_int, CStr, CString};
use std::os::raw::c_void;
use std::ptr;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
const EXEC_PERM: c_int = 0o100;

struct Result {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

fn create_result_string(op: &str, val: c_int) -> Option<CString> {
    let s = format!("Operation: {}, Value: {}", op, val);
    CString::new(s).ok()
}

fn check_permissions(perms: c_int, required: c_int) -> bool {
    (perms & required) == required
}

fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if !check_permissions(perms, READ_PERM | WRITE_PERM) {
        println!("Insufficient permissions for addition");
        return 0;
    }
    a + b
}

fn multiply_with_log(a: c_int, b: c_int, log_msg: &mut *mut c_char) -> c_int {
    let result = a * b;
    if let Some(cstr) = create_result_string("multiply", result) {
        let ptr = cstr.into_raw();
        *log_msg = ptr;
    } else {
        *log_msg = ptr::null_mut();
    }
    result
}

fn copy_and_sum(src: *const c_int, count: usize) -> c_int {
    if src.is_null() {
        println!("Source pointer is NULL");
        return -1;
    }

    let src_slice = unsafe { std::slice::from_raw_parts(src, count) };
    let dest: Vec<c_int> = src_slice.to_vec();

    dest.iter().sum()
}

fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        println!("One or both operation strings are NULL");
        return -1;
    }

    let cstr1 = unsafe { CStr::from_ptr(op1) };
    let cstr2 = unsafe { CStr::from_ptr(op2) };

    cstr1.cmp(cstr2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut log_message: *mut c_char = ptr::null_mut();

    let permissions: c_int = 0o644;

    let mut res_tracker = Box::new(Result {
        value: 0,
        operation: [0; 32],
        permissions,
    });

    let none_str = CString::new("none").unwrap();
    unsafe {
        ptr::copy_nonoverlapping(
            none_str.as_ptr(),
            res_tracker.operation.as_mut_ptr(),
            5,
        );
    }

    match mode {
        1 => {
            let op_str = CString::new("addition").unwrap();
            unsafe {
                ptr::copy_nonoverlapping(
                    op_str.as_ptr(),
                    res_tracker.operation.as_mut_ptr(),
                    9,
                );
            }
            result = safe_add(value1, value2, permissions);
            res_tracker.value = result;

            println!("Mode 1: Addition");
            println!("Result: {}", result);
        }

        2 => {
            let op_str = CString::new("multiplication").unwrap();
            unsafe {
                ptr::copy_nonoverlapping(
                    op_str.as_ptr(),
                    res_tracker.operation.as_mut_ptr(),
                    15,
                );
            }
            result = multiply_with_log(value1, value2, &mut log_message);
            res_tracker.value = result;

            if log_message.is_null() {
                println!("Log message creation failed");
            } else {
                let log_cstr = unsafe { CStr::from_ptr(log_message) };
                if log_cstr.to_bytes().is_empty() {
                    println!("Log message creation failed");
                } else {
                    println!("Mode 2: {:?}", log_cstr);
                    unsafe {
                        let _ = CString::from_raw(log_message);
                    }
                }
            }
        }

        3 => {
            let op_str = CString::new("array_sum").unwrap();
            unsafe {
                ptr::copy_nonoverlapping(
                    op_str.as_ptr(),
                    res_tracker.operation.as_mut_ptr(),
                    10,
                );
            }
            let values: [c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(values.as_ptr(), 3);
            res_tracker.value = result;

            println!("Mode 3: Array Sum");
            println!("Result: {}", result);
        }

        4 => {
            let op_str = CString::new("complex").unwrap();
            unsafe {
                ptr::copy_nonoverlapping(
                    op_str.as_ptr(),
                    res_tracker.operation.as_mut_ptr(),
                    8,
                );
            }

            if check_permissions(permissions, EXEC_PERM) {
                result = (value1 * value2) + value3;
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

    let op_cstr = unsafe {
        CStr::from_ptr(res_tracker.operation.as_ptr())
    };
    if op_cstr.to_bytes() != b"none" {
        println!("Operation performed: {:?}", op_cstr);
    }

    result
}
