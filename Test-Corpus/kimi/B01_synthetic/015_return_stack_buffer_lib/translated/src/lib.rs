use std::ffi::{CStr, c_char, c_int};
use std::os::raw::c_char as RawCChar;

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        if let Ok(s) = c_str.to_str() {
            println!("{}", s);
        }
    }
}

fn helper_bad() -> *const c_char {
    let char_string = "helperBad string\0";
    char_string.as_ptr() as *const c_char
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> &'static [u8] {
    b"helperGood1 string\0"
}

fn good() {
    let s = helper_good1();
    print_line(s.as_ptr() as *const c_char);
}
