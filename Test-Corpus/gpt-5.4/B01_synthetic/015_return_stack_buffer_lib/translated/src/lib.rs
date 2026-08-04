use std::ffi::{c_char, CStr};
use std::os::raw::c_int;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        println!("{}", c_str.to_string_lossy());
    }
}

fn helper_bad() -> *const c_char {
    static CHAR_STRING: &[u8] = b"helperBad string\0";
    CHAR_STRING.as_ptr() as *const c_char
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> *const c_char {
    static CHAR_STRING: &[u8] = b"helperGood1 string\0";
    CHAR_STRING.as_ptr() as *const c_char
}

fn good() {
    print_line(helper_good1());
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
