use std::ffi::{c_char, c_int, CStr};
use std::os::raw::c_void;
use std::ptr;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    unsafe {
        let str_ptr = malloc(64) as *mut c_char;
        if str_ptr.is_null() {
            return ptr::null_mut();
        }
        snprintf(
            str_ptr,
            64,
            c"Operation: %s, Value: %d".as_ptr(),
            op,
            val,
        );
        str_ptr
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    if (perms & required) == required {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        print!("Insufficient permissions for addition\n");
        return 0;
    }
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_log(a: c_int, b: c_int, log_msg: *mut *mut c_char) -> c_int {
    unsafe {
        let product = a.wrapping_mul(b);
        *log_msg = create_result_string(c"multiply".as_ptr(), product);
        if (*log_msg).is_null() {
            return 0;
        }
        product
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    unsafe {
        if src.is_null() {
            print!("Source pointer is NULL\n");
            return -1;
        }

        let size = count as usize * std::mem::size_of::<c_int>();
        let dest = malloc(size) as *mut c_int;
        if dest.is_null() {
            print!("Memory allocation failed\n");
            return -1;
        }

        memcpy(dest as *mut c_void, src as *const c_void, size);

        let mut sum: c_int = 0;
        for i in 0..count as usize {
            sum = sum.wrapping_add(*dest.add(i));
        }

        free(dest as *mut c_void);
        sum
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    unsafe {
        if op1.is_null() || op2.is_null() {
            print!("One or both operation strings are NULL\n");
            return -1;
        }
        strcmp(op1, op2)
    }
}

#[repr(C)]
struct Result {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    unsafe {
        #[allow(unused_assignments)]
        let mut result: c_int = 0;
        let mut log_message: *mut c_char = ptr::null_mut();

        let permissions: c_int = 0o644;

        let res_tracker = malloc(std::mem::size_of::<Result>()) as *mut Result;
        if res_tracker.is_null() {
            print!("Failed to allocate result tracker\n");
            return -1;
        }

        (*res_tracker).value = 0;
        (*res_tracker).permissions = permissions;
        strcpy((*res_tracker).operation.as_mut_ptr(), c"none".as_ptr());

        match mode {
            1 => {
                strcpy((*res_tracker).operation.as_mut_ptr(), c"addition".as_ptr());
                result = safe_add(value1, value2, permissions);
                (*res_tracker).value = result;

                print!("Mode 1: Addition\n");
                print!("Result: {}\n", result);
            }
            2 => {
                strcpy(
                    (*res_tracker).operation.as_mut_ptr(),
                    c"multiplication".as_ptr(),
                );
                result = multiply_with_log(value1, value2, &mut log_message);
                (*res_tracker).value = result;

                if log_message.is_null() {
                    print!("Log message creation failed\n");
                } else {
                    let cstr = CStr::from_ptr(log_message);
                    if cstr.to_bytes().is_empty() {
                        print!("Log message creation failed\n");
                    } else {
                        // Use printf-style to match C exactly: "Mode 2: %s\n"
                        print!("Mode 2: {}\n", cstr.to_str().unwrap_or(""));
                        free(log_message as *mut c_void);
                    }
                }
            }
            3 => {
                strcpy(
                    (*res_tracker).operation.as_mut_ptr(),
                    c"array_sum".as_ptr(),
                );
                let mut values: [c_int; 3] = [value1, value2, value3];
                result = copy_and_sum(values.as_mut_ptr(), 3);
                (*res_tracker).value = result;

                print!("Mode 3: Array Sum\n");
                print!("Result: {}\n", result);
            }
            4 => {
                strcpy(
                    (*res_tracker).operation.as_mut_ptr(),
                    c"complex".as_ptr(),
                );

                if check_permissions(permissions, 0o100) != 0 {
                    result = (value1.wrapping_mul(value2)).wrapping_add(value3);
                } else {
                    result = (value1.wrapping_add(value2)).wrapping_add(value3);
                }

                (*res_tracker).value = result;
                print!("Mode 4: Complex Calculation\n");
                print!("Result: {}\n", result);
            }
            _ => {
                print!("Invalid mode\n");
                result = -1;
            }
        }

        if strcmp((*res_tracker).operation.as_ptr(), c"none".as_ptr()) != 0 {
            let op = CStr::from_ptr((*res_tracker).operation.as_ptr());
            print!("Operation performed: {}\n", op.to_str().unwrap_or(""));
        }

        free(res_tracker as *mut c_void);

        result
    }
}
