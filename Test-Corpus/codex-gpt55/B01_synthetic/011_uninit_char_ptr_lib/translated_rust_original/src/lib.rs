#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let data: MaybeUninit<*const c_char> = MaybeUninit::uninit();
    printLine(unsafe { data.assume_init() });
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let data = c"string".as_ptr();
    printLine(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
