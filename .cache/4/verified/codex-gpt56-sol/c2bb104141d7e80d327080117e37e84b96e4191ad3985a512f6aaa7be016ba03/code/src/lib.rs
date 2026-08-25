use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(value: *const c_char) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let _ = puts(line);
    }
}

#[no_mangle]
pub extern "C" fn bad() {
    unsafe {
        printLine(c"bad()".as_ptr());
    }
}

fn helper_good() {
    unsafe {
        printLine(c"helperGood()".as_ptr());
    }
}

#[no_mangle]
pub extern "C" fn good() {
    unsafe {
        printLine(c"good()".as_ptr());
    }
    helper_good();
}

#[no_mangle]
pub extern "C" fn main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    unsafe {
        printLine(c"Calling good()...".as_ptr());
    }
    good();
    unsafe {
        printLine(c"Finished good()".as_ptr());
        printLine(c"Calling bad()...".as_ptr());
    }
    bad();
    unsafe {
        printLine(c"Finished bad()".as_ptr());
    }

    0
}
