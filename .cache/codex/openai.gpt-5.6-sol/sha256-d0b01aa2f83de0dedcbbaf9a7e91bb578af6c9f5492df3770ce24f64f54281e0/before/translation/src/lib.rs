use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    fn puts(s: *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    unsafe {
        printLine(ptr::null());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    unsafe {
        printLine(c"string".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        unsafe {
            good();
        }
    } else {
        unsafe {
            bad();
        }
    }
}
