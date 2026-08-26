use std::ffi::CStr;
use std::io::{self, Read};
use std::os::raw::{c_char, c_int};

fn print_line_impl(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap_or(""));
    }
}

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    print_line_impl(line);
}

fn helper_bad() -> *const c_char {
    // C returns address of local variable — UB. GCC optimizes to return NULL.
    std::ptr::null()
}

#[no_mangle]
pub extern "C" fn bad() {
    print_line_impl(helper_bad());
}

fn helper_good1() -> *const c_char {
    static CHAR_STRING: &[u8] = b"helperGood1 string\0";
    CHAR_STRING.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn good() {
    print_line_impl(helper_good1());
}

#[no_mangle]
pub extern "C" fn main() -> c_int {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x: c_int = input
        .trim_start()
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
