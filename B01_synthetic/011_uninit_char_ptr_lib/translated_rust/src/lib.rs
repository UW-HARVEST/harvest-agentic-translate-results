use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: *const c_char = MaybeUninit::uninit().assume_init();
    printLine(data);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    printLine(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    unsafe {
        if use_good != 0 {
            good();
        } else {
            bad();
        }
    }
}
