use std::ffi::{CStr, CString, c_char};
use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn cleanup(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let numbers = [a, b, c, d];
    let mut result: c_int = 0;

    let expected_str = c"VALID";
    let input_str = c"VALID";
    if input_str.to_bytes().len() < expected_str.to_bytes().len()
        || &input_str.to_bytes()[..expected_str.to_bytes().len()] != expected_str.to_bytes()
    {
        println!("Input string validation failed.");
        return result;
    }

    for number in numbers {
        match number {
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
                result += number;
            }
        }
    }

    let dynamic_str = CString::new("Processed numbers: numbers").ok();
    if dynamic_str.is_none() {
        println!("Memory allocation failed.");
        return result;
    }

    if let Some(s) = &dynamic_str {
        println!("{}", s.to_string_lossy());
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn print_result(label: *const c_char, result: c_int) {
    let label = if label.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(label) }.to_string_lossy().into_owned()
    };
    println!("{}: {}", label, result);
}

#[unsafe(no_mangle)]
pub extern "C" fn cleanup_resources(dynamic_str: *mut c_char) {
    if dynamic_str.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(dynamic_str);
    }
}
