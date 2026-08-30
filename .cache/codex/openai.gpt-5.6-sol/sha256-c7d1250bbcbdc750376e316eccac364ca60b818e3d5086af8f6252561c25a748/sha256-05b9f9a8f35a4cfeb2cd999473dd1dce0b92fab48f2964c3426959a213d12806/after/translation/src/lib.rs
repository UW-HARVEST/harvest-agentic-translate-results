use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(s: *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    unsafe {
        printLine(c"bad()".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    unsafe {
        printLine(c"good()".as_ptr());
        printLine(c"helperGood()".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    unsafe {
        printLine(c"Calling good()...".as_ptr());
        good();
        printLine(c"Finished good()".as_ptr());
        printLine(c"Calling bad()...".as_ptr());
        bad();
        printLine(c"Finished bad()".as_ptr());
    }
}
