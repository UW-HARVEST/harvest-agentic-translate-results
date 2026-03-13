#![allow(non_snake_case, static_mut_refs)]

use std::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

/// Intentionally returns a dangling pointer to a stack-local buffer,
/// reproducing the C undefined behavior exactly.
unsafe fn helper_bad() -> *mut c_char {
    let char_string: [u8; 17] = *b"helperBad string\0";
    char_string.as_ptr() as *mut c_char
}

fn helper_good1() -> *mut c_char {
    static mut CHAR_STRING: [u8; 19] = *b"helperGood1 string\0";
    unsafe { CHAR_STRING.as_mut_ptr() as *mut c_char }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(unsafe { helper_bad() });
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printLine(helper_good1());
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
