use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    fn puts(s: *const c_char) -> c_int;
}

static HELPER_GOOD1_STRING: &[u8] = b"helperGood1 string\0";

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

fn helper_bad() -> *const c_char {
    ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(helper_bad());
}

fn helper_good1() -> *const c_char {
    HELPER_GOOD1_STRING.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printLine(helper_good1());
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
