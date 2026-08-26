use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
}

static INTEGER_FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(INTEGER_FORMAT.as_ptr().cast(), int_number);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    unsafe {
        printIntLine(0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    unsafe {
        printIntLine(0);
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
