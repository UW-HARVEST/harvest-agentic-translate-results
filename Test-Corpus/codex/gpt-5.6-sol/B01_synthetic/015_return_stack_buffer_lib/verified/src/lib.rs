use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    fn puts(line: *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // GCC materializes the C function's invalid pointer return as null.
    printLine(ptr::null());
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    static CHAR_STRING: &[u8] = b"helperGood1 string\0";
    printLine(CHAR_STRING.as_ptr().cast());
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
