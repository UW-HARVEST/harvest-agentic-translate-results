use std::ffi::{c_char, c_int, c_void, CStr};

extern "C" {
    fn free(ptr: *mut c_void);
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers = [a, b, c, d];
    let mut result = 0;

    let expected_str = "VALID";
    let input_str = "VALID";
    if !input_str.starts_with(expected_str) {
        println!("Input string validation failed.");
        return result;
    }

    for &num in &numbers {
        match num {
            10 => {
                result += 10;
                result += 20;
            }
            20 => {
                result += 20;
            }
            30 => {
                result += 30;
                result += 40;
            }
            40 => {
                result += 40;
            }
            _ => {
                result += num;
            }
        }
    }

    println!("Processed numbers: numbers");

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn print_result(label: *const c_char, result: c_int) {
    if label.is_null() {
        return;
    }
    let c_str = unsafe { CStr::from_ptr(label) };
    println!("{}: {}", c_str.to_string_lossy(), result);
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe {
            free(dynamic_str as *mut c_void);
        }
    }
}
