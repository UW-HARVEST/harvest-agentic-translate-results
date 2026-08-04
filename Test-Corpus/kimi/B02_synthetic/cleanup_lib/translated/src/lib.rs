use std::ffi::{c_char, c_int, CStr};
use std::os::raw::c_char as c_char_t;

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers = [a, b, c, d];
    let mut result = 0;

    let expected_str = b"VALID\0";
    let input_str = b"VALID\0";
    if input_str != expected_str {
        eprintln!("Input string validation failed.");
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

    let dynamic_str = format!("Processed numbers: {:?}", numbers);
    println!("{}", dynamic_str);

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn print_result(label: *const c_char, result: c_int) {
    let label_str = unsafe {
        CStr::from_ptr(label).to_string_lossy()
    };
    println!("{}: {}", label_str, result);
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if !dynamic_str.is_null() {
        unsafe {
            let _ = Box::from_raw(dynamic_str);
        }
    }
}
