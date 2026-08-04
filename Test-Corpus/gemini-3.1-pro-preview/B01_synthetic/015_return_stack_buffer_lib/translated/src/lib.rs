use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            if let Ok(s) = CStr::from_ptr(line).to_str() {
                println!("{}", s);
            }
        }
    }
}

fn helper_bad() -> *mut c_char {
    let mut char_string = *b"helperBad string\0";
    char_string.as_mut_ptr() as *mut c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(helper_bad());
}

fn helper_good1() -> *mut c_char {
    static mut CHAR_STRING: [u8; 19] = *b"helperGood1 string\0";
    unsafe { std::ptr::addr_of_mut!(CHAR_STRING) as *mut c_char }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printLine(helper_good1());
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
