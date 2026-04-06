use std::ffi::{c_char, c_int, CStr};
use std::ptr;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;

#[unsafe(no_mangle)]
pub extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    let str_ptr = unsafe { libc::malloc(64) as *mut c_char };
    if str_ptr.is_null() {
        return ptr::null_mut();
    }
    let op_cstr = unsafe { CStr::from_ptr(op) };
    let op_str = op_cstr.to_str().unwrap_or("");
    let formatted = format!("Operation: {}, Value: {}", op_str, val);
    let bytes = formatted.as_bytes();
    let len = bytes.len().min(63);
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), str_ptr as *mut u8, len);
        *str_ptr.add(len) = 0;
    }
    str_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    if (perms & required) == required { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        print!("Insufficient permissions for addition\n");
        return 0;
    }
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_log(a: c_int, b: c_int, log_msg: *mut *mut c_char) -> c_int {
    let product = a * b;
    let op = std::ffi::CString::new("multiply").unwrap();
    let result_str = create_result_string(op.as_ptr(), product);
    unsafe { *log_msg = result_str; }
    if result_str.is_null() {
        return 0;
    }
    product
}

#[unsafe(no_mangle)]
pub extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    if src.is_null() {
        print!("Source pointer is NULL\n");
        return -1;
    }
    let count = count as usize;
    let dest = unsafe { libc::malloc(count * std::mem::size_of::<c_int>()) as *mut c_int };
    if dest.is_null() {
        print!("Memory allocation failed\n");
        return -1;
    }
    unsafe {
        ptr::copy_nonoverlapping(src, dest, count);
    }
    let mut sum: c_int = 0;
    for i in 0..count {
        sum += unsafe { *dest.add(i) };
    }
    unsafe { libc::free(dest as *mut _); }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    if op1.is_null() || op2.is_null() {
        print!("One or both operation strings are NULL\n");
        return -1;
    }
    unsafe { libc::strcmp(op1, op2) }
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    let result: c_int;
    let permissions: c_int = 0o644;
    let mut operation = "none";

    // Allocate result tracker (matching C behavior)
    let res_tracker = unsafe { libc::malloc(std::mem::size_of::<c_int>() * 2 + 32) };
    if res_tracker.is_null() {
        print!("Failed to allocate result tracker\n");
        return -1;
    }

    match mode {
        1 => {
            operation = "addition";
            result = safe_add(value1, value2, permissions);
            print!("Mode 1: Addition\n");
            print!("Result: {}\n", result);
        }
        2 => {
            operation = "multiplication";
            let mut log_message: *mut c_char = ptr::null_mut();
            result = multiply_with_log(value1, value2, &mut log_message);
            if log_message.is_null() {
                print!("Log message creation failed\n");
            } else {
                let msg = unsafe { CStr::from_ptr(log_message) };
                let msg_str = msg.to_str().unwrap_or("");
                if msg_str.is_empty() {
                    print!("Log message creation failed\n");
                } else {
                    print!("Mode 2: {}\n", msg_str);
                }
                unsafe { libc::free(log_message as *mut _); }
            }
        }
        3 => {
            operation = "array_sum";
            let mut values = [value1, value2, value3];
            result = copy_and_sum(values.as_mut_ptr(), 3);
            print!("Mode 3: Array Sum\n");
            print!("Result: {}\n", result);
        }
        4 => {
            operation = "complex";
            if check_permissions(permissions, 0o100) != 0 {
                result = (value1 * value2) + value3;
            } else {
                result = value1 + value2 + value3;
            }
            print!("Mode 4: Complex Calculation\n");
            print!("Result: {}\n", result);
        }
        _ => {
            print!("Invalid mode\n");
            result = -1;
        }
    }

    if operation != "none" {
        print!("Operation performed: {}\n", operation);
    }

    unsafe { libc::free(res_tracker); }
    result
}
