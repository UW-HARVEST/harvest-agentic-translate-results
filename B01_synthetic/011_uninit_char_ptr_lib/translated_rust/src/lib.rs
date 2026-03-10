use std::ffi::c_int;
use std::os::raw::c_char;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe { printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

fn bad() {
    unsafe {
        let data: *const c_char = std::mem::MaybeUninit::uninit().assume_init();
        print_line(data);
    }
}

fn good() {
    unsafe {
        let data: *const c_char = b"string\0".as_ptr() as *const c_char;
        print_line(data);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
