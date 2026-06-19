use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static HELPER_GOOD1_STRING: [u8; 19] = *b"helperGood1 string\0";
static PRINT_LINE_FORMAT: [u8; 4] = *b"%s\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(PRINT_LINE_FORMAT.as_ptr().cast(), line);
        }
    }
}

fn helper_bad() -> *mut c_char {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(helper_bad());
}

fn helper_good1() -> *mut c_char {
    HELPER_GOOD1_STRING.as_ptr().cast_mut().cast()
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
