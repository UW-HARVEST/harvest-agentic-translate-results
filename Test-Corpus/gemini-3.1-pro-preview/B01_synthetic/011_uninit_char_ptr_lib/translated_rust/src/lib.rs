use std::ffi::{c_char, CStr};
use std::os::raw::c_int;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        println!("{}", c_str.to_string_lossy());
    }
}

fn bad() {
    let data: *const c_char = std::ptr::null();
    print_line(data);
}

fn good() {
    let data: *const c_char = c"string".as_ptr();
    print_line(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
