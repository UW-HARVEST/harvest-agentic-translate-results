use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap());
    }
}

fn helper_good() {
    printLine(b"helperGood()\0".as_ptr() as *const c_char);
}

#[no_mangle]
pub extern "C" fn good() {
    printLine(b"good()\0".as_ptr() as *const c_char);
    helper_good();
}

#[no_mangle]
pub extern "C" fn bad() {
    printLine(b"bad()\0".as_ptr() as *const c_char);
}

#[no_mangle]
pub extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good();
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad();
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
    0
}
