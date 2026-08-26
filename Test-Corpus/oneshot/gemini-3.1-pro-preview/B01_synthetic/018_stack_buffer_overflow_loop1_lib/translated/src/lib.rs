use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            if let Ok(s) = CStr::from_ptr(line).to_str() {
                println!("{}", s);
            }
        }
    }
}

fn print_int_line(int_number: c_int) {
    println!("{}", int_number);
}

fn bad() {
    let mut data = [0 as c_int; 10];
    let source = [0 as c_int; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    let mut data = [0 as c_int; 10];
    let source = [0 as c_int; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
