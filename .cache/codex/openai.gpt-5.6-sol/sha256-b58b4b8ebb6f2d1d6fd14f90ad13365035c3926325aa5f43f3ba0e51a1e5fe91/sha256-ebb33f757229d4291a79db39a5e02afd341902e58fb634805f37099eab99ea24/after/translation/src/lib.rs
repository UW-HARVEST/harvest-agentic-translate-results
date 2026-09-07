use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    fn puts(value: *const c_char) -> c_int;
}

static GOOD_STRING: [c_char; 19] = [
    b'h' as c_char,
    b'e' as c_char,
    b'l' as c_char,
    b'p' as c_char,
    b'e' as c_char,
    b'r' as c_char,
    b'G' as c_char,
    b'o' as c_char,
    b'o' as c_char,
    b'd' as c_char,
    b'1' as c_char,
    b' ' as c_char,
    b's' as c_char,
    b't' as c_char,
    b'r' as c_char,
    b'i' as c_char,
    b'n' as c_char,
    b'g' as c_char,
    0,
];

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
        printLine(GOOD_STRING.as_ptr());
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
