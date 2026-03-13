use std::ffi::c_int;
use std::mem::MaybeUninit;
use std::os::raw::c_char;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

unsafe fn bad() {
    let data: *const c_char = unsafe { MaybeUninit::uninit().assume_init() };
    unsafe {
        print_line(data);
    }
}

fn good() {
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    unsafe {
        print_line(data);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        unsafe {
            bad();
        }
    }
}
