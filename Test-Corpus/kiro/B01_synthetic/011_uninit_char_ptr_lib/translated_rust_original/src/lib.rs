use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

unsafe fn bad() {
    let data: *const c_char = MaybeUninit::uninit().assume_init();
    print_line(data);
}

unsafe fn good() {
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    print_line(data);
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
